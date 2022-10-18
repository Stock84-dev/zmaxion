use std::{
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut, Range},
    pin::Pin,
    sync::Arc,
};

use async_trait::async_trait;
use zmaxion_core::{models::TopicRef, prelude::*};
use zmaxion_param::{
    IntoParamStateMut, ParamBuilder, PipeObserver, PipeParam, PipeParamFetch, PipeParamImpl,
    PipeParamState, PipeParamStateImpl, TopicParam, TopicParamKind,
};
use zmaxion_rt::{AsyncMutex, AsyncMutexGuard};
use zmaxion_utils::prelude::*;

use crate::{
    bevy_ecs::system::Resource,
    components::TopicRwBuilderId,
    resources::{TopicReaderBuilders, TopicWriterBuilders},
    TopicFeatures,
};

/// 2 pointer dereferences for reader abstraction + probably 1 more for async readers to get to
/// Vec<T>
/// 1 + 1 ponter dereferences if there is nothing to read
#[derive(Deref, Component)]
pub struct DynReaderState<T>(Arc<AsyncMutex<Box<dyn ReaderState<T>>>>);
#[derive(Deref, Component)]
pub struct DynWriterState<T>(Arc<AsyncMutex<Box<dyn WriterState<T>>>>);
/// Increases arc pointer
impl<T> Clone for DynWriterState<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Clone for DynReaderState<T> {
    /// Increases arc pointer
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[async_trait]
pub trait ReaderState<T>: Send + Sync {
    /// The caller must not call this function multiple times so that topic read semantics of a pipe
    /// can withhold. Can return and Err if Pipe is despawning.
    async unsafe fn read<'a>(&'a mut self) -> AnyResult<ReadGuard<'a, T>>;
    /// The caller must not call this function multiple times so that topic read semantics of a pipe
    /// can withhold.
    unsafe fn try_read<'a>(&'a mut self) -> Option<ReadGuard<'a, T>>;
}

#[async_trait]
pub trait TopicReader<'a, T>: Sized + Send + Sync {
    // If this method fails inside a pipe, then it should get forwarded
    async fn read(self) -> AnyResult<ReadGuard<'a, T>>;
    fn try_read(self) -> Option<ReadGuard<'a, T>>;
}

#[async_trait]
impl<'s, T, P: Send + Sync> TopicReader<'s, T> for P
where
    T: Resource,
    P: IntoParamStateMut<'s> + 's,
    P::State: ReaderState<T> + 's,
{
    async fn read(mut self) -> AnyResult<ReadGuard<'s, T>> {
        unsafe { self.into_state_mut().read().await }
    }

    fn try_read(mut self) -> Option<ReadGuard<'s, T>> {
        unsafe { self.into_state_mut().try_read() }
    }
}

#[async_trait]
impl<T: Resource> PipeParamState for DynReaderState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Reader;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.command().pipeline_id;
        Ok(DynReaderState::clone(
            builder.world().await.get::<DynReaderState<T>>(id).unwrap(),
        ))
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for DynReaderState<T> {
    type Item = DynReader<'s, T>;

    fn get_param(&'s mut self) -> Self::Item {
        DynReader { state: self }
    }
}

impl<'s, T: Resource> PipeParam for DynReader<'s, T> {
    type State = DynReaderState<T>;
}

pub struct DynReader<'a, T> {
    state: &'a DynReaderState<T>,
}

impl<'a, T> DynReader<'a, T> {
    async fn lock(mut self) -> AsyncMutexGuard<'a, Box<dyn ReaderState<T>>> {
        self.state.lock().await
    }

    fn try_lock(mut self) -> Option<AsyncMutexGuard<'a, Box<dyn ReaderState<T>>>> {
        self.state.try_lock()
    }
}

#[async_trait]
impl<'s, T> ReaderState<T> for AsyncMutexGuard<'s, Box<dyn ReaderState<T>>> {
    async unsafe fn read<'a>(&'a mut self) -> AnyResult<ReadGuard<'a, T>> {
        self.deref_mut().read().await
    }

    unsafe fn try_read<'a>(&'a mut self) -> Option<ReadGuard<'a, T>> {
        self.deref_mut().try_read()
    }
}

#[async_trait]
pub trait TopicWriter<T>: Send + Sync {
    fn extend_one(self, value: T);
    fn extend_from_slice(self, data: &[T])
    where
        T: Clone;
    fn extend<I>(self, iter: I)
    where
        I: IntoIterator<Item = T>;
    fn extend_dyn(self, iter: &mut dyn Iterator<Item = T>);
}

#[async_trait]
pub trait WriterState<T>: Send + Sync {
    unsafe fn extend_one(&mut self, value: T);
    unsafe fn extend_from_slice(&mut self, data: &[T])
    where
        T: Clone;
    unsafe fn extend<I>(&mut self, iter: I)
    where
        Self: Sized,
        I: IntoIterator<Item = T>;
    unsafe fn extend_dyn(&mut self, iter: &mut dyn Iterator<Item = T>);
}

#[async_trait]
impl<'s, T, P: Send + Sync> TopicWriter<T> for P
where
    T: Resource,
    P: IntoParamStateMut<'s>,
    P::State: WriterState<T> + 's,
{
    fn extend_one(self, value: T) {
        unsafe {
            self.into_state_mut().extend_one(value);
        }
    }

    fn extend_from_slice(self, data: &[T])
    where
        T: Clone,
    {
        unsafe {
            self.into_state_mut().extend_from_slice(data);
        }
    }

    fn extend<I>(self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        unsafe { self.into_state_mut().extend(iter) }
    }

    fn extend_dyn(self, iter: &mut dyn Iterator<Item = T>) {
        unsafe { self.into_state_mut().extend(iter) }
    }
}

#[async_trait]
impl<T: Resource> PipeParamState for DynWriterState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Writer;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.command().pipeline_id;
        Ok(DynWriterState::clone(
            builder.world().await.get::<DynWriterState<T>>(id).unwrap(),
        ))
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for DynWriterState<T> {
    type Item = DynWriter<'s, T>;

    fn get_param(&'s mut self) -> Self::Item {
        DynWriter { state: self }
    }
}

impl<'s, T: Resource> PipeParam for DynWriter<'s, T> {
    type State = DynWriterState<T>;
}

pub struct DynWriter<'a, T> {
    state: &'a DynWriterState<T>,
}

impl<'a, T> DynWriter<'a, T> {
    async fn lock(mut self) -> AsyncMutexGuard<'a, Box<dyn WriterState<T>>> {
        self.state.lock().await
    }

    fn try_lock(mut self) -> Option<AsyncMutexGuard<'a, Box<dyn WriterState<T>>>> {
        self.state.try_lock()
    }
}

#[async_trait]
impl<'a, T> WriterState<T> for AsyncMutexGuard<'a, Box<dyn WriterState<T>>> {
    unsafe fn extend_from_slice(&mut self, data: &[T])
    where
        T: Clone,
    {
        self.deref_mut().extend_from_slice(data)
    }

    unsafe fn extend_one(&mut self, value: T) {
        self.deref_mut().extend_one(value)
    }

    unsafe fn extend<I>(&mut self, iter: I)
    where
        Self: Sized,
        I: IntoIterator<Item = T>,
    {
        let mut iter = iter.into_iter();
        self.deref_mut().extend_dyn(&mut iter);
    }

    unsafe fn extend_dyn(&mut self, iter: &mut dyn Iterator<Item = T>) {
        self.deref_mut().extend_dyn(iter);
    }
}

pub enum GuardedEvents<'a, T> {
    Guard(SpinRwLockReadGuard<'a, Vec<T>>, Range<usize>),
    Slice(&'a [T]),
}

pub struct ReadGuard<'a, T> {
    guard: GuardedEvents<'a, T>,
}

impl<'a, T> ReadGuard<'a, T> {
    pub fn new(guard: GuardedEvents<'a, T>) -> Self {
        Self { guard }
    }
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match &self.guard {
            GuardedEvents::Guard(guard, range) => &guard[range.clone()],
            GuardedEvents::Slice(slice) => slice,
        }
    }
}
