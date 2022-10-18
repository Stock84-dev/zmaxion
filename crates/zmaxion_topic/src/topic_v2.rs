use std::{
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut, Range},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use ergnomics::prelude::*;
use futures_util::{
    future,
    future::{ready, Ready},
    FutureExt,
};
use zmaxion_core::flow::{Message, MessageFetch, MessagePool, Poolfetch, ToEventWriter};
use zmaxion_flow::{Event, EventRange, EventsAll};
use zmaxion_utils::pool::{PoolArc, PoolArcMut, PoolItem, POOL};

use crate::{Resource, __export__::AnyResult};

struct DfEvents {
    rows: Vec<Event>,
    columns: Vec<EventsAll>,
    column_ranges: Vec<EventRange>,
}

pub struct SystemDfTopic<Meta> {
    read: DfEvents,
    writer: DfEvents,
    _t: PhantomData<Meta>,
}

impl<PoolFetcher> SystemDfTopic<PoolFetcher> {
    pub fn write_guard<'a, E>(&'a mut self) -> SystemDfTopicWriteGuard<'a, E, PoolFetcher>
    where
        PoolFetcher: Poolfetch<'a, E>,
    {
        let pool = PoolFetcher::create();
        SystemDfTopicWriteGuard {
            events: &mut self.writer,
            pool,
            batch_start: 0,
            _t: Default::default(),
        }
    }

    pub fn try_read<'a, E>(&'a self) -> Option<SystemDfTopicGuard<'a, E, PoolFetcher>> {
        if self.read.columns.is_empty()
            && self.read.column_ranges.is_empty()
            && self.read.rows.is_empty()
        {
            return None;
        }
        Some(SystemDfTopicGuard {
            events: &self.read,
            _t: Default::default(),
        })
    }
}

fn a(read: &SystemDfTopic<AckedFetcher>, write: &mut SystemDfTopic<AckedFetcher>) {
    let mut read = read.try_read::<i32>().unwrap();
    let mut guard = write.write_guard::<u64>();
    read.iter().for_each(|x| {
        let transformed = x.transformer().map(x, |x| *x as u64);
        guard.push(transformed);
    });
    guard.write_all();
    // Flow::from::<i32>(read).map(|x| *x as u64).write(writer)
}

trait FlowItem {
    type Event;
}

struct FlowStruct<R, E> {
    reader: R,
    _t: PhantomData<E>,
}

pub struct JoinAsyncFuture<Inner, O> {
    inner: Inner,
    _t: PhantomData<O>,
}

fn flow() -> Flow<(), ()> {
    Flow {
        readers: (),
        writers: (),
    }
}

pub struct SystemDfTopicWriteGuard<'a, E, Meta: Poolfetch<'a, E>> {
    events: &'a mut DfEvents,
    pool: PoolArcMut<Meta::Pool>,
    batch_start: usize,
    _t: PhantomData<(E, Meta)>,
}

impl<'a, E, Meta> SystemDfTopicWriteGuard<'a, E, Meta>
where
    Meta: Poolfetch<'a, E>,
    Meta::Pool: MessagePool,
{
    pub fn push(&mut self, msg: <Meta::Pool as MessagePool>::Message) {
        self.pool.push(msg);
    }

    pub fn new_batch(&mut self) {
        self.batch_start = self.pool.len();
    }

    pub fn write_one(&mut self) {
        // SAFETY: messages are read on the next epoch or a write lock is aquired
        let pool = unsafe { self.pool.to_dyn_shared() };
        self.events.rows.push(Event {
            data: pool,
            row: self.batch_start as u32,
        });
    }

    pub fn write_range(&mut self) {
        // SAFETY: messages are read on the next epoch or a write lock is aquired
        let pool = unsafe { self.pool.to_dyn_shared() };
        self.events.column_ranges.push(EventRange {
            start: self.batch_start as u32,
            end: self.pool.len() as u32,
            data: pool,
        });
    }

    pub fn write_all(&mut self) {
        // SAFETY: messages are read on the next epoch or a write lock is aquired
        let pool = unsafe { self.pool.to_dyn_shared() };
        self.events.columns.push(EventsAll { data: pool });
    }
}

pub struct SystemDfTopicGuard<'a, M, Meta> {
    events: &'a DfEvents,
    _t: PhantomData<(M, Meta)>,
}
pub struct Flow<Readers, Writers> {
    readers: Readers,
    writers: Writers,
}

impl<'a, T: Resource> MessageFetch<'a> for AckPool<T> {
    type Message = AckedMessage<'a, T>;

    fn len(&self) -> usize {
        self.data.len()
    }

    fn at(&'a self, at: u32) -> Self::Message {
        AckedMessage {
            id: at as usize,
            pool: self,
        }
    }
}

impl<Readers, Writers> Flow<Readers, Writers> {
    fn from<R, E>(self, reader: R) -> Flow<(FlowStruct<R, E>, Readers), Writers> {
        Flow {
            readers: (
                FlowStruct {
                    reader,
                    _t: PhantomData::<E>::default(),
                },
                self.readers,
            ),
            writers: self.writers,
        }
    }
}

impl<'a, E: Resource> Poolfetch<'a, E> for AckedFetcher {
    type Pool = AckPool<E>;

    fn fetch(data: &'a PoolArc<dyn PoolItem>) -> &'a Self::Pool {
        let pool: &AckPool<E> = data
            .downcast_ref::<Self::Pool>()
            .expect("Invalid message data store");
        pool
    }

    fn create() -> PoolArcMut<Self::Pool> {
        POOL.get::<Self::Pool>()
    }
}
impl<'a, E: Resource, Fetcher> SystemDfTopicGuard<'a, E, Fetcher>
where
    Fetcher: Poolfetch<'a, E>,
{
    fn iter(
        &'a self,
    ) -> impl Iterator<Item = <<Fetcher as Poolfetch<'a, E>>::Pool as MessageFetch<'a>>::Message>
    {
        self.events
            .columns
            .iter()
            .map(|x: &EventsAll| {
                let pool = Fetcher::fetch(&x.data);
                (0..pool.len()).into_iter().map(|at| pool.at(at as u32))
            })
            .flatten()
            .chain(
                self.events
                    .column_ranges
                    .iter()
                    .map(|x: &EventRange| {
                        let pool = Fetcher::fetch(&x.data);
                        (x.start..x.end).into_iter().map(|at| pool.at(at as u32))
                    })
                    .flatten(),
            )
            .chain(self.events.rows.iter().map(|x: &Event| {
                let pool = Fetcher::fetch(&x.data);
                pool.at(x.row)
            }))
    }
}

struct AckedFetcher;

struct AckPool<E> {
    data: Vec<E>,
    acks: Vec<AckData>,
}

impl<M: Resource> PoolItem for AckPool<M> {
    fn reset(&mut self) {
        self.data.clear();
        self.acks.clear();
    }
}

struct OwnedAckMessage<E> {
    data: E,
    acks: AckData,
}

impl<E> Message for OwnedAckMessage<E> {
    type Transformer = ();
    type Value = E;

    fn value(&self) -> &Self::Value {
        todo!()
    }

    fn transformer(&self) -> Self::Transformer {
        todo!()
    }

    //    fn trace_data(&self) -> Option<&TraceData> {
    //        todo!()
    //    }
    //
    //    fn ack_data(&self) -> Option<&AckData> {
    //        todo!()
    //    }
    //
    //    fn trace_data_mut(&self) -> Option<&mut TraceData> {
    //        todo!()
    //    }
    //
    //    fn ack_data_mut(&self) -> Option<&mut AckData> {
    //        todo!()
    //    }
    //
    //    fn join<T: Message>(self, other: T) -> JoinedMessage<Self, T>
    //    where
    //        Self: Sized,
    //    {
    //        todo!()
    //    }
}

impl<T: Resource> MessagePool for AckPool<T> {
    type Message = OwnedAckMessage<T>;

    fn push(&mut self, message: Self::Message) {
        self.data.push(message.data);
        self.acks.push(message.acks);
    }
}

struct AckedMessage<'a, T> {
    id: usize,
    pool: &'a AckPool<T>,
}

trait System {}
struct Wrapper<T>(T);

// impl<T> System for SystemDfTopic<T> {
//}
// impl<T> System for SystemTopic<Wrapper<T>> {
//}

pub struct MainVec<T> {
    data: Vec<T>,
    ack_data: Option<Vec<AckData>>,
    trace_data: Option<Vec<TraceData>>,
}

impl<T: Resource> PoolItem for MainVec<T> {
    fn reset(&mut self) {
        self.data.clear();
        self.ack_data.some_mut(|x| x.clear());
        self.trace_data.some_mut(|x| x.clear());
    }
}

struct TraceData;
#[derive(Clone)]
struct AckData;
struct RefAckTransformer;

impl RefAckTransformer {
    fn map<'a, F, A, B>(&self, a: AckedMessage<'a, A>, mapper: F) -> OwnedAckMessage<B>
    where
        F: FnOnce(&A) -> B,
    {
        let acks = a.pool.acks[a.id].clone();
        OwnedAckMessage {
            data: (mapper)(a.value()),
            acks,
        }
    }
}

impl<'a, E> Message for AckedMessage<'a, E> {
    type Transformer = RefAckTransformer;
    type Value = E;

    fn value(&self) -> &Self::Value {
        todo!()
    }

    fn transformer(&self) -> Self::Transformer {
        todo!()
    }

    //    fn trace_data(&self) -> Option<&TraceData> {
    //        todo!()
    //    }
    //
    //    fn ack_data(&self) -> Option<&AckData> {
    //        todo!()
    //    }
    //
    //    fn trace_data_mut(&self) -> Option<&mut TraceData> {
    //        todo!()
    //    }
    //
    //    fn ack_data_mut(&self) -> Option<&mut AckData> {
    //        todo!()
    //    }
    //
    //    fn join<T: Message>(self, other: T) -> JoinedMessage<Self, T>
    //    where
    //        Self: Sized,
    //    {
    //        todo!()
    //    }
}

struct JoinedMessage<A, B> {
    a: A,
    b: B,
}

// impl<V: Resource> Message<V> for Row {
//    fn value(&self) -> &V {
//        let arc = self.data.deref();
//        let values: &Vec<V> = match arc.downcast_ref::<Vec<V>>() {
//            None => {
//                panic!("Invalid message data store.");
//            }
//            Some(values) => values,
//        };
//        &values[self.row as usize]
//    }
//
//    fn trace_data(&self) -> Option<&TraceData> {
//        todo!()
//    }
//
//    fn ack_data(&self) -> Option<&AckData> {
//        todo!()
//    }
//
//    fn trace_data_mut(&self) -> Option<&mut TraceData> {
//        todo!()
//    }
//
//    fn ack_data_mut(&self) -> Option<&mut AckData> {
//        todo!()
//    }
//
//    fn join<T: Message<U>, U>(self, other: T) -> JoinedMessage<Self, T> {
//        todo!()
//    }
//}
