use std::{
    future::Future,
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{
    future,
    future::{ready, Ready},
    FutureExt,
};
use zmaxion_utils::{
    pool::{PoolArc, PoolArcMut, PoolItem, POOL},
    prelude::AnyResult,
};

use crate::{flower::Flower, Event, EventRange, EventsAll};

pub trait Poolfetch<'a, E>: Send + Sync + 'static {
    type Pool: MessageFetch<'a> + PoolItem;
    fn fetch(data: &'a PoolArc<dyn PoolItem>) -> &'a Self::Pool;
    fn create() -> PoolArcMut<Self::Pool>;
}

pub trait ToFlower<'f> {
    type Flower: 'f;
    fn to_flower(&'f mut self) -> AnyResult<Self::Flower>;
}

pub trait EventValue {
    type Tuple;
    fn into_tuple(self) -> Self::Tuple;
}

impl<'f, T: 'f + Flower<'f>> ToFlower<'f> for T {
    type Flower = &'f mut T;

    fn to_flower(&'f mut self) -> AnyResult<Self::Flower> {
        Ok(self)
    }
}

pub trait ToAsyncFlower<'f> {
    type Flower: 'f;
    type Future: Future<Output = AnyResult<Self::Flower>>;
    fn to_async_flower(&'f mut self) -> Self::Future;
}

impl<'f, T: ToFlower<'f>> ToAsyncFlower<'f> for T {
    type Flower = T::Flower;
    type Future = Ready<AnyResult<T::Flower>>;

    fn to_async_flower(&'f mut self) -> Self::Future {
        futures_util::future::ready(self.to_flower())
    }
}

// pub trait IntoBatcher<'p, P, E> {
//    type Batcher;
//    type Future: Future<Output = AnyResult<Self::Batcher>>;
//    fn into_batcher(&'p mut self) -> Self::Future;
//}

// pub struct Batcher<'a, P: Poolfetch<'a, E>, W: EventConsumer, E> {
//    writer: W,
//    pool: PoolArcMut<P::Pool>,
//    _t: PhantomData<P>,
//}
// impl<'a, P: Poolfetch<'a, E>, W: EventConsumer, E> Batcher<'a, P, W, E>
// where
//    P::Pool: MessagePool,
//{
//    pub fn push(&mut self, event: <P::Pool as MessagePool>::Message) {
//        self.pool.push(event);
//    }
//}
// impl<'a, P: Poolfetch<'a, E>, W: EventConsumer, E> Drop for Batcher<'a, P, W, E> {
//    fn drop(&mut self) {
//        self.writer.write_all(EventsAll {
//            data: unsafe { self.pool.to_dyn_shared() },
//        });
//    }
//}

// impl<'p, T: ToAsyncEventWriter<'p, P, E>, P, E> IntoBatcher<'p, P, E> for T
// where
//    T::Future: 'p,
//    T::Writer: EventConsumer,
//    P: Poolfetch<'p, E>,
//{
//    type Batcher = Batcher<'p, P, T::Writer, E>;
//    type Future = future::Map<
//        <T as ToAsyncEventWriter<'p, P, E>>::Future,
//        fn(
//            <<T as ToAsyncEventWriter<'p, P, E>>::Future as Future>::Output,
//        ) -> AnyResult<Batcher<'p, P, T::Writer, E>>,
//    >;
//
//    fn into_batcher(&'p mut self) -> Self::Future {
//        self.to_async_event_writer().map(|x| {
//            x.map(|x| Batcher {
//                writer: x,
//                pool: P::create(),
//                _t: Default::default(),
//            })
//        })
//    }
//}

pub trait EventConsumer {
    fn write_one(&mut self, event: Event);
    fn write_range(&mut self, event: EventRange);
    fn write_all(&mut self, event: EventsAll);
}

pub trait ToEventWriter<P, E> {
    type Writer: EventConsumer;
    fn to_event_writer(&mut self) -> AnyResult<Self::Writer>;
}

pub trait MessageFetch<'a> {
    type Message: Message<'a> + 'a;
    fn len(&self) -> usize;
    fn at(&'a self, at: u32, dyn_pool: &'a PoolArc<dyn PoolItem>) -> Self::Message;
}

pub trait MessagePool<'a>: PoolItem {
    type Message: Message<'a>;
    fn push(&mut self, message: Self::Message);
}

pub trait MapableMessage<'a, E, O> {
    type Mapped: Message<'a>;
    fn map<F: FnMut(E) -> O>(self, f: F) -> Self::Mapped;
}

pub trait PoolBuilder<M>: Sized + PoolItem {
    fn create() -> Option<PoolArcMut<Self>>;
    fn push(&mut self, message: M);
    fn len(&self) -> usize;
}

pub trait Message<'a>: Sized {
    type Value: 'static;
    type Pool: PoolBuilder<Self>;
    type PooledMessage: Message<'a>;
    const IS_OWNED: bool;
    fn dyn_pool(&self) -> Option<&PoolArc<dyn PoolItem>>;
    fn id(&self) -> u32;
    fn into_pooled(
        self,
        pool: &'a mut Option<PoolArcMut<Self::Pool>>,
        dyn_pool: &'a Option<PoolArc<dyn PoolItem>>,
    ) -> Self::PooledMessage;

    //    type Transformer;
    //    type Value;
    // NOTE: downcast is not optimized because it is a trait obejct and anything can change behind
    // it TOOD: move outside of loop
    //    fn value(&self) -> &Self::Value;
    //    fn transformer(&self) -> Self::Transformer;
    //    fn trace_data(&self) -> Option<&TraceData>;
    //    fn ack_data(&self) -> Option<&AckData>;
    //    fn trace_data_mut(&self) -> Option<&mut TraceData>;
    //    fn ack_data_mut(&self) -> Option<&mut AckData>;
    //    fn join<T: Message>(self, other: T) -> JoinedMessage<Self, T>
    //    where
    //        Self: Sized;
}

pub trait ToAsyncEventWriter<P, E: 'static> {
    type Future: Future<Output = AnyResult<Self::Writer>>;
    type Writer: EventConsumer;
    fn to_async_event_writer(&mut self) -> Self::Future;
}

impl<T: ToEventWriter<P, E>, P, E: 'static> ToAsyncEventWriter<P, E> for T {
    type Future = Ready<AnyResult<T::Writer>>;
    type Writer = T::Writer;

    fn to_async_event_writer(&mut self) -> Self::Future {
        ready(self.to_event_writer())
    }
}

pub struct PooledMessage<'a, M: PoolItem> {
    pub pool: &'a PoolArcMut<M>,
    pub id: usize,
}

pub trait Pooled {
    type Message;
}

impl<'a, M: PoolItem> Pooled for PooledMessage<'a, M> {
    type Message = Self;
}
