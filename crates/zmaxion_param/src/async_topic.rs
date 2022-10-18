use std::{
    ops::{Deref, Range},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bevy_ecs::system::SystemParam;
use cache_padded::CachePadded;
use ergnomics::prelude::*;
use zmaxion_core::{
    bevy_ecs::{
        archetype::Archetype,
        system::{Resource, SystemMeta, SystemParamFetch, SystemParamState},
    },
    prelude::*,
};
use zmaxion_rt::prelude::*;
use zmaxion_utils::prelude::*;

use crate::{
    prelude::*, IntoParamStateMut, ParamBuilder, PipeObserver, PipeParam, PipeParamFetch,
    PipeParamImpl, PipeParamState, PipeParamStateImpl, TopicParamKind,
};

#[derive(Component)]
pub struct AsyncTopic<T>(Arc<AsyncTopicInner<T>>);

impl<T: Resource> AsyncTopic<T> {
    pub fn update_system(query: Query<&AsyncTopic<T>>) {
        for q in query.iter() {
            q.update();
        }
    }
}

impl<T> Clone for AsyncTopic<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for AsyncTopic<T> {
    type Target = AsyncTopicInner<T>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: Resource> Default for AsyncTopic<T> {
    fn default() -> Self {
        Self(Arc::default())
    }
}

pub struct AsyncTopicInner<T> {
    /// cumulative number of all events processed + number of events in read buffer
    n_total_readable_events: AtomicU64,
    notify: Notify,
    write_events: CachePadded<SpinMutex<Vec<T>>>,
    read_events: CachePadded<SpinRwLock<Vec<T>>>,
    reader_cursors: SpinRwLock<Vec<CachePadded<Option<AtomicU64>>>>,
}

impl<T: Resource> Default for AsyncTopicInner<T> {
    fn default() -> Self {
        AsyncTopicInner {
            n_total_readable_events: Default::default(),
            notify: Default::default(),
            write_events: Default::default(),
            read_events: Default::default(),
            reader_cursors: Default::default(),
        }
    }
}

impl<T: Resource> AsyncTopicInner<T> {
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
}

#[derive(new)]
pub struct AsyncReader<'a, T: Resource>(&'a mut AsyncReaderState<T>);

impl<T: Resource> AsyncReaderState<T> {
    pub async fn read_raw<'a>(&'a self) -> (SpinRwLockReadGuard<'a, Vec<T>>, Range<usize>) {
        let reader_cursor;
        {
            let guard = self.topic.reader_cursors.read();
            reader_cursor = guard
                .get(self.reader_id)
                .expect_with(|| format!("Reader `{}` not subscribed", self.reader_id))
                .as_ref()
                .expect_with(|| format!("Reader `{}` not subscribed", self.reader_id))
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

    pub fn try_read_raw<'a>(&'a self) -> (SpinRwLockReadGuard<'a, Vec<T>>, Range<usize>) {
        let reader_cursor;
        {
            let guard = self.topic.reader_cursors.read();
            reader_cursor = guard
                .get(self.reader_id)
                .expect_with(|| format!("Reader `{}` not subscribed", self.reader_id))
                .as_ref()
                .expect_with(|| format!("Reader `{}` not subscribed", self.reader_id))
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
pub struct AsyncReaderState<T: Resource> {
    topic: AsyncTopic<T>,
    n_writer_references: Arc<()>,
    reader_id: usize,
}

impl<T: Resource> Drop for AsyncReaderState<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.n_writer_references) == 1 {
            self.topic.despawn_reader(self.reader_id);
        }
    }
}

pub struct AsyncWriter<'a, T: Resource>(&'a mut AsyncWriterState<T>);

#[derive(Component)]
pub struct AsyncWriterState<T: Resource> {
    topic: AsyncTopic<T>,
}

impl<T: Resource> AsyncWriterState<T> {
    pub unsafe fn extend_from_slice(&self, data: &[T])
    where
        T: Clone,
    {
        self.topic.write_events.lock().extend_from_slice(data);
    }

    pub unsafe fn extend_one(&self, value: T) {
        self.topic.write_events.lock().push(value);
    }

    pub unsafe fn extend<I: IntoIterator<Item = T>>(&self, iter: I) {
        self.topic.write_events.lock().extend(iter);
    }

    pub fn topic(&self) -> &AsyncTopic<T> {
        &self.topic
    }
}

impl<'a, T: Resource> PipeParam for AsyncReader<'a, T> {
    type State = AsyncReaderState<T>;
}

#[async_trait]
impl<T: Resource> PipeParamState for AsyncReaderState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Reader;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world().await;
        let topic = world.try_ref::<AsyncTopic<T>>(id)?;
        Ok(Self::from(topic.clone()))
    }
}

impl<T: Resource> From<AsyncTopic<T>> for AsyncReaderState<T> {
    fn from(topic: AsyncTopic<T>) -> Self {
        let reader_id = topic.new_reader();
        Self {
            reader_id,
            topic,
            n_writer_references: Arc::new(()),
        }
    }
}

impl<T: Resource> PipeObserver for AsyncReaderState<T> {
}

impl<'s, T: Resource> IntoParamStateMut<'s> for AsyncReader<'s, T> {
    fn into_state_mut(self) -> &'s mut Self::State {
        self.0
    }
}

impl<'s, T: Resource> IntoParamStateMut<'s> for AsyncWriter<'s, T> {
    fn into_state_mut(self) -> &'s mut Self::State {
        self.0
    }
}

impl<T: Resource> From<AsyncTopic<T>> for AsyncWriterState<T> {
    fn from(topic: AsyncTopic<T>) -> Self {
        Self { topic }
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for AsyncReaderState<T> {
    type Item = AsyncReader<'s, T>;

    fn get_param(&'s mut self) -> Self::Item {
        AsyncReader(self)
    }
}

impl<'a, T: Resource> PipeParam for AsyncWriter<'a, T> {
    type State = AsyncWriterState<T>;
}

#[async_trait]
impl<T: Resource> PipeParamState for AsyncWriterState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Writer;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world().await;
        let topic = world.try_ref::<AsyncTopic<T>>(id)?;
        Ok(Self::from(topic.clone()))
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for AsyncWriterState<T> {
    type Item = AsyncWriter<'s, T>;

    fn get_param(&'s mut self) -> Self::Item {
        AsyncWriter(self)
    }
}

impl<T: Resource> PipeObserver for AsyncWriterState<T> {
}
