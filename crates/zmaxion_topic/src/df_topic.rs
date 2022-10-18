use std::{
    ops::Range,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use bevy_reflect::TypeRegistryArc;
use cache_padded::CachePadded;
use polars::{
    chunked_array::ChunkedArray,
    datatypes::*,
    prelude::{ChunkOps, DataFrame, NamedFrom, PolarsNumericType, Series},
    series::IntoSeries,
};
use zmaxion_rt::Notify;
use zmaxion_utils::prelude::{SpinMutex, SpinRwLock, *};

pub struct DfTopic(Arc<DfTopicInner>);

#[derive(Default)]
pub struct DfTopicInner {
    read: DataFrame,
    write: SpinMutex<DataFrame>,
    registry: TypeRegistryArc,
}

impl DfTopicInner {
    fn read(&self) {
        let series = &self.read[0];
        let column = series.0.u32().unwrap();
        if !column.is_optimal_aligned() {}
    }
}

pub struct PolarsMessage<I> {
    topic: DfTopic,
    index: I,
}

pub trait RawTopic: for<'a> GuardedRawTopic<'a> + Default {
    fn read_lock<'a>(&'a self) -> <Self as GuardedRawTopic<'a>>::ReadGuard;
    // must return none if there are no elements inside
    fn write_lock<'a>(&'a self) -> Option<<Self as GuardedRawTopic<'a>>::WriteGuard>;
    fn n_events_to_read(&self) -> usize;
    // SAFETY: we have checked if all readers have finished, which means they will not access
    // data. This is enforced through other methods.
    unsafe fn update(&self, guard: &mut <Self as GuardedRawTopic<'_>>::WriteGuard);
}

pub trait GuardedRawTopic<'a> {
    type WriteGuard: 'a;
    type ReadGuard: 'a;
}

impl<'a> GuardedRawTopic<'a> for DfTopicInner {
    type ReadGuard = &'a DataFrame;
    type WriteGuard = SpinMutexGuard<'a, DataFrame>;
}

impl RawTopic for DfTopicInner {
    fn read_lock<'a>(&'a self) -> <Self as GuardedRawTopic<'a>>::ReadGuard {
        &self.read
    }

    fn write_lock<'a>(&'a self) -> Option<<Self as GuardedRawTopic<'a>>::WriteGuard> {
        let guard = self.write.lock();
        if guard.is_empty() {
            return None;
        }
        Some(guard)
    }

    fn n_events_to_read(&self) -> usize {
        self.read.height()
    }

    unsafe fn update(&self, write_events: &mut <Self as GuardedRawTopic<'_>>::WriteGuard) {
        let read_events = &mut *(&self.read as *const _ as *mut _);
        std::mem::swap(read_events, &mut *write_events);
        let garbage = write_events;
        // OPT: unfortunatelly polars doesn't have a clear method
        for w in 0..read_events.width() {
            let name = read_events[w].name();
            let dtype = read_events[w].dtype();
            let column = &mut read_events.get_columns_mut()[w];
            fn rechunk<T: PolarsNumericType>(name: &str, series: &mut Series)
            where
                ChunkedArray<T>: IntoSeries,
            {
                let chunked: &mut ChunkedArray<T> = series.as_mut();
                if !chunked.is_optimal_aligned() {
                    let rechunked = chunked.rechunk();
                    *series = Series::new(name, rechunked)
                }
            }
            let series = match column.dtype() {
                DataType::Boolean => rechunk::<BooleanType>(name, column),
                DataType::UInt8 => rechunk::<UInt8Type>(name, column),
                DataType::UInt16 => rechunk::<UInt16Type>(name, column),
                DataType::UInt32 => rechunk::<UInt32Type>(name, column),
                DataType::UInt64 => rechunk::<UInt64Type>(name, column),
                DataType::Int8 => rechunk::<Int8Type>(name, column),
                DataType::Int16 => rechunk::<Int16Type>(name, column),
                DataType::Int32 => rechunk::<Int32Type>(name, column),
                DataType::Int64 => rechunk::<Int64Type>(name, column),
                DataType::Float32 => rechunk::<Float32Type>(name, column),
                DataType::Float64 => rechunk::<Float64Type>(name, column),
                DataType::Utf8 => rechunk::<Utf8Type>(name, column),
                DataType::Date => rechunk::<DateType>(name, column),
                DataType::Datetime(_, _) => rechunk::<DatetimeType>(name, column),
                DataType::Duration(_) => rechunk::<DurationType>(name, column),
                DataType::Time => rechunk::<TimeType>(name, column),
                DataType::List(_) => rechunk::<ListType>(name, column),
                DataType::Object(kind) => {
                    let reg = self.registry.read().get_with_name(kind).unwrap();
                    rechunk::<ObjectType>(name, column)
                }
                DataType::Null => {}
                DataType::Categorical(_) => rechunk::<CategoricalType>(name, column),
                DataType::Struct(_) => rechunk::<StructType>(name, column),
                DataType::Unknown => {}
            };
            let series = Series::new_empty(name, dtype);
            if !column.is_optimal_aligned() {}
            column.is_o * column = series;
        }
    }
}

pub struct HighThroghputRawTopicWrapper<T> {
    group_cursors: Cursors,
    raw: T,
}

impl<T: RawTopic> HighThroghputRawTopicWrapper<T> {
    pub fn update(&self) {
        let mut guard = match self.raw.write_lock() {
            None => return,
            Some(guard) => guard,
        };
        // this guard must hold for the entire stack, a new reader could subscribe while we are
        // updatng and could have its cursor initialized before we do swap but later theere are
        // no events so it will always lag and lead to no progress
        let cursors = self.group_cursors.groups.read();
        if !self.group_cursors.all_on_latest(&cursors) {
            return;
        }
        unsafe {
            self.raw.update(&mut guard);
        }
    }

    pub fn new_group(&self) -> usize {
        self.group_cursors.new_group(self.raw.n_events_to_read())
    }

    pub async fn read<'a>(
        &'a self,
        group: usize,
    ) -> Result<RawReadGuard<<T as GuardedRawTopic<'a>>::ReadGuard>, TopicError> {
        let cursor;
        {
            let guard = self.raw.read_lock();
            cursor = self.group_cursors.load(group)?;
        }
        if cursor == self.group_cursors.current_max.load(Ordering::SeqCst) {
            // wait for new events
            self.group_cursors.notify.notified().await;
        }
        let guard = self.raw.read_lock();
        Ok(guard)
    }

    fn read_all<'a, 'b: 'a>(
        &self,
        guard: &'b SpinRwLockReadGuard<'a, Vec<T>>,
        reader_id: usize,
        cursor: u64,
    ) -> Range<usize> {
        //        debug!("{:#?}", &self.topic.as *const _);
        //        debug!("{:#?}", self.topic.reader_cursors);
        //        debug!("{:#?}", self.reader_cursor);

        let n_events = self.n_total_readable_events.load(Ordering::SeqCst);
        let len = guard.len();
        let range = len - (n_events - cursor) as usize..len;
        self.reader_cursors
            .read()
            .get(reader_id)
            .expect_with(|| format!("Reader `{}` not subscribed", reader_id))
            .as_ref()
            .expect_with(|| format!("Reader `{}` not subscirbed", reader_id))
            .fetch_add(guard.len() as u64, Ordering::SeqCst);
        //        debug!("{:#?}", self.topic.reader_cursors);
        //        debug!("{:#?}", self.reader_cursor);
        if !range.is_empty() {
            trace!("{}::read_all()", std::any::type_name::<Self>());
        }
        range
    }
}

pub struct Cursors {
    groups: SpinRwLock<Vec<CachePadded<Option<AtomicU64>>>>,
    current_max: AtomicU64,
    notify: Notify,
}

impl Cursors {
    pub fn load(&self, group: usize) -> Result<u64, TopicError> {
        self.map(group, |x| x.load(Ordering::SeqCst))
    }

    pub fn fetch_add(&self, group: usize, amount: u64) -> Result<u64, TopicError> {
        self.map(group, |x| x.fetch_add(amount, Ordering::SeqCst))
    }

    fn map<F: FnOnce(&AtomicU64) -> T, T>(&self, group: usize, f: F) -> Result<T, TopicError> {
        self.groups
            .read()
            .get(group)
            .ok_or(TopicError::InvalidGroup(group))
            .and_then(|x| x.as_ref().ok_or(TopicError::InvalidGroup(group)).map(f))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TopicError {
    #[error("Reader group `{0}` is invalid.")]
    InvalidGroup(usize),
}

impl Cursors {
    fn all_on_latest(
        &self,
        groups: &SpinRwLockReadGuard<Vec<CachePadded<Option<AtomicU64>>>>,
    ) -> bool {
        let n_events = self.current_max.load(Ordering::SeqCst);
        // wait for all groups to read all items
        groups
            .iter()
            .filter_map(|x| x.as_ref())
            .all(|x| x.load(Ordering::SeqCst) == n_events)
    }

    fn update<T: 'static>(&self, n_new_events: usize) {
        trace!("{}::update()", std::any::type_name::<Self>());
        self.current_max
            .fetch_add(n_new_events as u64, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn new_group(&self, n_events_to_read: usize) -> usize {
        let mut cursors = self.groups.write();
        let start = self.current_max.load(Ordering::SeqCst) - n_events_to_read as u64;
        let cursor = CachePadded::new(Some(AtomicU64::new(start)));
        match cursors.iter().position(|x| x.is_none()) {
            None => {
                let i = cursors.len();
                cursors.push(cursor);
                i
            }
            Some(i) => {
                cursors[i] = cursor;
                i
            }
        }
    }
}

pub struct GenericReadGuard<'a, T> {
    read_guard: T,
    groups: SpinRwLockReadGuard<'a, Vec<CachePadded<Option<AtomicU64>>>>,
    cursors: &'a Cursors,
    group: usize,
}

pub trait RawReadGuard {
    type Item;
    type Items;
    fn read_one(&self, current_max: u64, at: u64) -> Self::Item;
    fn read_range(&self, current_max: u64, start: u64, at_least_len: usize) -> Self::Items;
}

impl<'a, T: RawReadGuard> GenericReadGuard<'a, T> {
    pub fn read_one(&self) -> Result<T::Item, TopicError> {
        let at = self.cursors.fetch_add(self.group, 1)? + 1;
        Ok(self.read_guard.read_one(at))
    }

    pub fn read_range(&self, len: usize) -> Self::Items {
    }
}

pub struct RangeReadGuard<'a, T> {
    guard: SpinRwLockReadGuard<'a, Vec<T>>,
    range: Range<usize>,
}
