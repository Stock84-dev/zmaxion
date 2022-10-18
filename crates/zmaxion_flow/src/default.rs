use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub use bevy_derive::{Deref, DerefMut};
use ergnomics::OptionExt;
use zmaxion_utils::pool::{PoolArc, PoolArcMut, PoolItem, POOL};

use crate::{
    flow::{MapableMessage, Message, MessageFetch, MessagePool, PoolBuilder, Poolfetch},
    Event,
};

pub struct DefaultPoolFetcher;

pub struct DefaultPool<E>(Vec<E>);

impl<'a, E: Send + Sync + 'static> PoolBuilder<DefaultOwnedMessage<'a, E>> for DefaultPool<E> {
    fn create() -> Option<PoolArcMut<Self>> {
        Some(POOL.get::<DefaultPool<E>>())
    }

    fn push(&mut self, message: DefaultOwnedMessage<'a, E>) {
        self.0.push(message.value)
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, E: Send + Sync + 'static> PoolBuilder<DefaultMessage<'a, E>> for DefaultPool<E> {
    fn create() -> Option<PoolArcMut<Self>> {
        Some(POOL.get::<DefaultPool<E>>())
    }

    fn push(&mut self, message: DefaultMessage<'a, E>) {
        todo!()
        //        self.0.push(message.value)
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

pub struct DefaultMessage<'a, T: Send + Sync + 'static> {
    id: u32,
    pool: &'a DefaultPool<T>,
    dyn_pool: &'a PoolArc<dyn PoolItem>,
}

impl<'a, T: Send + Sync + 'static> MessageFetch<'a> for DefaultPool<T> {
    type Message = DefaultMessage<'a, T>;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn at(&'a self, at: u32, dyn_pool: &'a PoolArc<dyn PoolItem>) -> Self::Message {
        DefaultMessage {
            id: at,
            pool: &self,
            dyn_pool,
        }
    }
}
impl<'a, T: Send + Sync + 'static> Message<'a> for DefaultMessage<'a, T> {
    type Pool = DefaultPool<T>;
    type PooledMessage = DefaultMessage<'a, T>;
    type Value = T;

    const IS_OWNED: bool = false;

    fn dyn_pool(&self) -> Option<&PoolArc<dyn PoolItem>> {
        Some(self.dyn_pool)
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn into_pooled(
        self,
        pool: &'a mut Option<PoolArcMut<Self::Pool>>,
        dyn_pool: &'a Option<PoolArc<dyn PoolItem>>,
    ) -> Self::PooledMessage {
        self
    }
}

impl<'a, T: Send + Sync + 'static> Deref for DefaultMessage<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.pool.0[self.id as usize]
    }
}

impl<E: Send + Sync + 'static> PoolItem for DefaultPool<E> {
    fn reset(&mut self) {
        self.0.clear();
    }
}

impl<'s, E: Send + Sync + 'static> Poolfetch<'s, E> for DefaultPoolFetcher {
    type Pool = DefaultPool<E>;

    fn fetch(data: &'s PoolArc<dyn PoolItem>) -> &'s Self::Pool {
        let pool = data
            .downcast_ref::<Self::Pool>()
            .expect("Invalid message data store");
        pool
    }

    fn create() -> PoolArcMut<Self::Pool> {
        POOL.get::<DefaultPool<E>>()
    }
}
impl<'a, E: Send + Sync + 'static, O: Send + Sync + 'static> MapableMessage<'a, &'a E, O>
    for DefaultMessage<'a, E>
{
    type Mapped = DefaultOwnedMessage<'a, O>;

    fn map<F: FnMut(&'a E) -> O>(self, mut f: F) -> Self::Mapped {
        let value = &self.pool.0[self.id as usize];
        DefaultOwnedMessage {
            value: f(value),
            _t: Default::default(),
        }
    }
}
pub struct DefaultOwnedMessage<'a, E> {
    value: E,
    _t: PhantomData<&'a ()>,
}

impl<'a, E: Send + Sync + 'static, O: Send + Sync + 'static> MapableMessage<'a, E, O>
    for DefaultOwnedMessage<'a, E>
{
    type Mapped = DefaultOwnedMessage<'a, O>;

    fn map<F: FnMut(E) -> O>(self, mut f: F) -> Self::Mapped {
        DefaultOwnedMessage {
            value: f(self.value),
            _t: Default::default(),
        }
    }
}

impl<'a, E: Send + Sync + 'static> Message<'a> for DefaultOwnedMessage<'a, E> {
    type Pool = DefaultPool<E>;
    type PooledMessage = DefaultMessage<'a, E>;
    type Value = E;

    const IS_OWNED: bool = true;

    fn dyn_pool(&self) -> Option<&PoolArc<dyn PoolItem>> {
        None
    }

    fn id(&self) -> u32 {
        panic!();
    }

    fn into_pooled(
        self,
        pool: &'a mut Option<PoolArcMut<Self::Pool>>,
        dyn_pool: &'a Option<PoolArc<dyn PoolItem>>,
    ) -> Self::PooledMessage {
        let pool = pool.as_mut().unwrap();
        pool.0.push(self.value);
        DefaultMessage {
            id: MessageFetch::len(&**pool) as u32 - 1,
            pool: unsafe { pool.as_shared() },
            dyn_pool: dyn_pool.as_ref().unwrap(),
        }
    }
}
impl<'a, E> Deref for DefaultOwnedMessage<'a, E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
// struct NoPool<'a, T>(PhantomData<&'a T>);
//
// impl<'a, T: Send + Sync + 'static> PoolItem for NoPool<'a, T> {
//    fn reset(&mut self) {
//    }
//}
// impl<'a, T: Send + Sync + 'static> MessagePool for NoPool<'a, T> {
//    type Message = DefaultMessage<'a, T>;
//
//    fn push(&mut self, message: Self::Message) {
//        todo!()
//    }
//}
