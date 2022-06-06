// 324
use std::{
    ops::Range,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use cache_padded::CachePadded;
use zmaxion_core::{
    bevy_ecs::{
        archetype::Archetype,
        system::{Resource, SystemMeta, SystemParamFetch, SystemParamState},
    },
    components::Arcc,
    prelude::*,
};
use zmaxion_rt::prelude::*;
use zmaxion_utils::prelude::*;

use crate::{
    prelude::*, ParamBuilder, PipeObserver, PipeParam, PipeParamFetch, PipeParamImpl,
    PipeParamState, PipeParamStateImpl, TopicParamKind,
};

pub struct AsyncTopic<T> {
    /// cumulative number of all events processed + number of events in read buffer
    n_total_readable_events: AtomicU64,
    notify: Notify,
    write_events: CachePadded<SpinMutex<Vec<T>>>,
    read_events: CachePadded<SpinRwLock<Vec<T>>>,
    reader_cursors: SpinRwLock<Vec<CachePadded<Option<AtomicU64>>>>,
}

impl<T: Resource> Default for AsyncTopic<T> {
    fn default() -> Self {
        AsyncTopic {
            n_total_readable_events: Default::default(),
            notify: Default::default(),
            write_events: Default::default(),
            read_events: Default::default(),
            reader_cursors: Default::default(),
        }
    }
}

impl<T: Resource> AsyncTopic<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_reader(&self) -> usize {
        let mut cursors = self.reader_cursors.write();
        let start = self.n_total_readable_events.load(Ordering::SeqCst)
            - self.read_events.read().len() as u64;
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
            .unwrap()
            .as_ref()
            .unwrap()
            .fetch_add(guard.len() as u64, Ordering::SeqCst);
        //        debug!("{:#?}", self.topic.reader_cursors);
        //        debug!("{:#?}", self.reader_cursor);
        if !range.is_empty() {
            trace!("{}::read_all()", std::any::type_name::<Self>());
        }
        range
    }

    pub fn write_slice(&mut self, data: &[T])
    where
        T: Clone,
    {
        self.write_events.lock().extend_from_slice(data)
    }

    pub fn write(&mut self, value: T) {
        self.write_events.lock().push(value);
    }

    pub fn update(&self) {
        let mut write_events = self.write_events.lock();
        if write_events.is_empty() {
            return;
        }
        let guard = self.reader_cursors.read();
        let n_events = self.n_total_readable_events.load(Ordering::SeqCst);
        if !guard
            .iter()
            .filter_map(|x| x.as_ref())
            .all(|x| x.load(Ordering::SeqCst) == n_events)
        {
            return;
        }
        if !write_events.is_empty() {
            trace!("{}::update()", std::any::type_name::<Self>());
        }
        let mut read_events = self.read_events.write();
        std::mem::swap(&mut *read_events, &mut *write_events);
        write_events.clear();
        self.n_total_readable_events
            .fetch_add(read_events.len() as u64, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn despawn_reader(&self, reader_id: usize) {
        let mut guard = self.reader_cursors.write();
        if reader_id < guard.len() {
            guard[reader_id] = CachePadded::new(None);
        }
    }

    pub fn update_system(query: Query<&Arcc<AsyncTopic<T>>>) {
        for q in query.iter() {
            q.update();
        }
    }
}

#[derive(Deref, DerefMut)]
pub struct AsyncTopicReader<'a, T: Resource>(&'a mut AsyncTopicReaderState<T>);

impl<T: Resource> AsyncTopicReaderState<T> {
    pub async fn read_raw<'a>(&'a mut self) -> (SpinRwLockReadGuard<'a, Vec<T>>, Range<usize>) {
        let reader_cursor;
        {
            let guard = self.topic.reader_cursors.read();
            reader_cursor = guard
                .get(self.reader_id)
                .unwrap()
                .as_ref()
                .unwrap()
                .load(Ordering::SeqCst);
        }
        if reader_cursor == self.topic.n_total_readable_events.load(Ordering::SeqCst) {
            // wait for new events
            self.topic.notify.notified().await;
        }
        let guard = self.topic.read_events.read();
        let range = self.topic.read_all(&guard, self.reader_id, reader_cursor);
        (guard, range)
    }

    pub fn try_read_raw<'a>(&'a mut self) -> (SpinRwLockReadGuard<'a, Vec<T>>, Range<usize>) {
        let reader_cursor;
        {
            let guard = self.topic.reader_cursors.read();
            reader_cursor = guard
                .get(self.reader_id)
                .unwrap()
                .as_ref()
                .unwrap()
                .load(Ordering::SeqCst);
        }
        let guard = self.topic.read_events.read();
        if reader_cursor == self.topic.n_total_readable_events.load(Ordering::SeqCst) {
            return (guard, Range::default());
        }
        let range = self.topic.read_all(&guard, self.reader_id, reader_cursor);
        (guard, range)
    }
}

#[derive(Component)]
pub struct AsyncTopicReaderState<T: Resource> {
    topic: Arc<AsyncTopic<T>>,
    n_writer_references: Arc<()>,
    reader_id: usize,
}

impl<T: Resource> Drop for AsyncTopicReaderState<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.n_writer_references) == 1 {
            self.topic.despawn_reader(self.reader_id);
        }
    }
}

pub struct AsyncTopicWriter<'a, T: Resource>(&'a mut AsyncTopicWriterState<T>);

#[derive(Component)]
pub struct AsyncTopicWriterState<T: Resource> {
    topic: Arc<AsyncTopic<T>>,
}

impl<T: Resource> AsyncTopicWriterState<T> {
    pub fn write_slice_raw(&self, data: &[T])
    where
        T: Clone,
    {
        self.topic.write_events.lock().extend_from_slice(data)
    }

    pub fn write_raw(&self, value: T) {
        self.topic.write_events.lock().push(value);
    }

    pub fn topic(&self) -> &Arc<AsyncTopic<T>> {
        &self.topic
    }
}

impl<'a, T: Resource> PipeParam for AsyncTopicReader<'a, T> {
    type State = AsyncTopicReaderState<T>;
}

#[async_trait]
impl<T: Resource> PipeParamState for AsyncTopicReaderState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Reader;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world();
        let topic = world.get::<Arcc<AsyncTopic<T>>>(id).unwrap();
        Ok(Self::from(Arc::clone(&*topic)))
    }
}

impl<T: Resource> From<Arc<AsyncTopic<T>>> for AsyncTopicReaderState<T> {
    fn from(topic: Arc<AsyncTopic<T>>) -> Self {
        let reader_id = topic.new_reader();
        Self {
            reader_id,
            topic,
            n_writer_references: Arc::new(()),
        }
    }
}

impl<T: Resource> From<Arc<AsyncTopic<T>>> for AsyncTopicWriterState<T> {
    fn from(topic: Arc<AsyncTopic<T>>) -> Self {
        Self { topic }
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for AsyncTopicReaderState<T> {
    type Item = AsyncTopicReader<'s, T>;

    fn get_param(&'s mut self) -> Self::Item {
        AsyncTopicReader(self)
    }
}

impl<'a, T: Resource> PipeParam for AsyncTopicWriter<'a, T> {
    type State = AsyncTopicWriterState<T>;
}

#[async_trait]
impl<T: Resource> PipeParamState for AsyncTopicWriterState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Writer;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world();
        let topic = world.get::<Arcc<AsyncTopic<T>>>(id).unwrap();
        Ok(Self::from(Arc::clone(&*topic)))
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for AsyncTopicWriterState<T> {
    type Item = AsyncTopicWriter<'s, T>;

    fn get_param(&'s mut self) -> Self::Item {
        AsyncTopicWriter(self)
    }
}
