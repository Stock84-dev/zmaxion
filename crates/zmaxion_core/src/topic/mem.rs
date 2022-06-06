use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};

use bevy::{
    ecs::system::{Resource, SystemMeta, SystemParamFetch, SystemParamState},
    log::trace,
    prelude::*,
    utils::HashMap,
};
use cache_padded::CachePadded;
pub use reader::{ResTopicReader, ResTopicReaderState, TopicReader, TopicReaderState};
use spin::RwLockReadGuard;
pub use writer::{ResTopicWriter, ResTopicWriterState, TopicWriter, TopicWriterState};
use zmaxion_rt::Notify;
use zmaxion_utils::some;

use crate::pipe::param::{PipeParam, TopicParam, TopicParamKind};

mod reader;
mod writer;

pub struct MemTopicInner<T: Resource> {
    /// cumulative number of all events processed + number of events in read buffer
    n_total_readable_events: AtomicU64,
    notify: Notify,
    write_events: CachePadded<spin::Mutex<Vec<T>>>,
    read_events: CachePadded<spin::RwLock<Vec<T>>>,
    reader_cursors: spin::RwLock<Vec<CachePadded<Option<AtomicU64>>>>,
    n_writers: AtomicUsize,
}

#[derive(Component)]
pub struct MemTopic<T: Resource>(pub Arc<MemTopicInner<T>>);

impl<T: Resource> Default for MemTopic<T> {
    fn default() -> Self {
        Self(Arc::new(MemTopicInner {
            n_total_readable_events: Default::default(),
            notify: Default::default(),
            write_events: Default::default(),
            read_events: Default::default(),
            reader_cursors: Default::default(),
            n_writers: Default::default(),
        }))
    }
}

impl<T: Resource> MemTopic<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_reader(&self) -> usize {
        let mut cursors = self.0.reader_cursors.write();
        let start = self.0.n_total_readable_events.load(Ordering::SeqCst)
            - self.0.read_events.read().len() as u64;
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

    pub fn register_writer(&self) {
        self.0.n_writers.fetch_add(1, Ordering::SeqCst);
    }

    pub fn despawn_writer(&self) {
        let n = self.0.n_writers.fetch_sub(1, Ordering::SeqCst);
        debug_assert_ne!(n, 0)
    }

    pub async fn read<'a>(&'a self, reader_id: usize) -> TopicReadGuard<'a, T> {
        let reader_cursor;
        {
            let guard = self.0.reader_cursors.read();
            reader_cursor = guard
                .get(reader_id)
                .unwrap()
                .as_ref()
                .unwrap()
                .load(Ordering::SeqCst);
        }
        if reader_cursor == self.0.n_total_readable_events.load(Ordering::SeqCst) {
            // wait for new events
            self.0.notify.notified().await;
        }
        TopicReadGuard {
            reader_cursor,
            reader_id,
            topic: self,
            guard: self.0.read_events.read(),
        }
    }

    pub fn try_read<'a>(&'a self, reader_id: usize) -> Option<TopicReadGuard<'a, T>> {
        let reader_cursor;
        {
            let guard = self.0.reader_cursors.read();
            reader_cursor = guard
                .get(reader_id)
                .unwrap()
                .as_ref()
                .unwrap()
                .load(Ordering::SeqCst);
        }
        if reader_cursor == self.0.n_total_readable_events.load(Ordering::SeqCst) {
            return None;
        }
        Some(TopicReadGuard {
            reader_cursor,
            reader_id,
            topic: self,
            guard: self.0.read_events.read(),
        })
    }

    pub fn write(&self, item: T) {
        self.write_all([item]);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        let mut iter = items.into_iter();
        let event = some!(iter.next());
        trace!("{}::write()", std::any::type_name::<Self>());
        let mut events = self.0.write_events.lock();
        events.push(event);
        events.extend(iter);
    }

    pub fn update(&self) {
        let mut write_events = self.0.write_events.lock();
        if write_events.is_empty() {
            return;
        }
        let guard = self.0.reader_cursors.read();
        let n_events = self.0.n_total_readable_events.load(Ordering::SeqCst);
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
        let mut read_events = self.0.read_events.write();
        std::mem::swap(&mut *read_events, &mut *write_events);
        write_events.clear();
        self.0
            .n_total_readable_events
            .fetch_add(read_events.len() as u64, Ordering::SeqCst);
        self.0.notify.notify_waiters();
    }

    pub fn consume(&self, reader_id: usize, n_items: usize) -> u64 {
        let prev = self
            .0
            .reader_cursors
            .read()
            .get(reader_id)
            .unwrap()
            .as_ref()
            .unwrap()
            .fetch_add(n_items as u64, Ordering::SeqCst);
        let cursor = prev + n_items as u64;
        cursor
    }

    pub fn despawn_reader(&self, reader_id: usize) {
        let mut guard = self.0.reader_cursors.write();
        if reader_id < guard.len() {
            guard[reader_id] = CachePadded::new(None);
        }
    }

    pub fn update_system(query: Query<&MemTopic<T>>) {
        for q in query.iter() {
            q.update();
        }
    }
}

impl<T: Resource> Clone for MemTopic<T> {
    /// Increases arc
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct TopicReadGuard<'a, T: Resource> {
    reader_id: usize,
    reader_cursor: u64,
    topic: &'a MemTopic<T>,
    guard: RwLockReadGuard<'a, Vec<T>>,
}

impl<'a, T: Resource> TopicReadGuard<'a, T> {
    pub fn read_all(&mut self) -> &[T] {
        //        debug!("{:#?}", &self.topic.0 as *const _);
        //        debug!("{:#?}", self.topic.0.reader_cursors);
        //        debug!("{:#?}", self.reader_cursor);
        let values = peek_all(&self.topic, &self.guard, self.reader_cursor);
        self.reader_cursor = self.topic.consume(self.reader_id, values.len());
        //        debug!("{:#?}", self.topic.0.reader_cursors);
        //        debug!("{:#?}", self.reader_cursor);
        if !values.is_empty() {
            trace!("{}::read_all()", std::any::type_name::<Self>());
        }
        values
    }

    pub fn try_read(&mut self) -> Option<&T> {
        trace!("{}::try_read()", std::any::type_name::<Self>());
        let value = peek(&self.topic, &self.guard, self.reader_cursor);
        self.reader_cursor = self.topic.consume(self.reader_id, 1);
        value
    }

    pub fn peek_all(&self) -> &[T] {
        peek_all(&self.topic, &self.guard, self.reader_cursor)
    }

    pub fn peek(&self) -> Option<&T> {
        peek(&self.topic, &self.guard, self.reader_cursor)
    }

    pub fn consume(&mut self, n_items: usize) {
        self.reader_cursor = self.topic.consume(self.reader_id, n_items);
    }
}

// satisfy borrow checker
fn peek<'a, T: Send + Sync + 'static>(
    topic: &'a MemTopic<T>,
    guard: &'a RwLockReadGuard<'a, Vec<T>>,
    reader_cursor: u64,
) -> Option<&'a T> {
    let n_events = topic.0.n_total_readable_events.load(Ordering::SeqCst);
    let len = guard.len();
    let to_read = guard.get(len - (n_events - reader_cursor) as usize);
    to_read
}

// satisfy borrow checker
fn peek_all<'a, T: Send + Sync + 'static>(
    topic: &'a MemTopic<T>,
    guard: &'a RwLockReadGuard<'a, Vec<T>>,
    reader_cursor: u64,
) -> &'a [T] {
    let n_events = topic.0.n_total_readable_events.load(Ordering::SeqCst);
    let len = guard.len();
    &guard[len - (n_events - reader_cursor) as usize..]
}
