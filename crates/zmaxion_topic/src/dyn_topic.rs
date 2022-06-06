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
    ControlFlow, ParamBuilder, PipeObserver, PipeParam, PipeParamFetch, PipeParamImpl,
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
    /// can withhold.
    async unsafe fn read<'a>(&'a mut self) -> ReadGuard<'a, T>;
    /// The caller must not call this function multiple times so that topic read semantics of a pipe
    /// can withhold.
    unsafe fn try_read<'a>(&'a mut self) -> ReadGuard<'a, T>;
}

#[async_trait]
pub trait TopicReader<'a, T>: Sized + Send + Sync {
    async fn read(self) -> ReadGuard<'a, T>;
    fn try_read(self) -> ReadGuard<'a, T>;
}

pub trait AsStateMut<'s> {
    type Mut;
    fn as_mut(&mut self) -> &'s mut Self::Mut;
}

#[async_trait]
impl<'s, T, P: Send + Sync> TopicReader<'s, T> for P
where
    T: Resource,
    P: PipeParam + AsStateMut<'s, Mut = P::State> + 's,
    P::State: ReaderState<T> + 's,
{
    async fn read(mut self) -> ReadGuard<'s, T> {
        unsafe { self.as_mut().read().await }
    }

    fn try_read(mut self) -> ReadGuard<'s, T> {
        unsafe { self.as_mut().try_read() }
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
            builder.world().get::<DynReaderState<T>>(id).unwrap(),
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
impl<'a, T> TopicReader<'a, T> for AsyncMutexGuard<'a, Box<dyn ReaderState<T>>> {
    async fn read(mut self) -> ReadGuard<'a, T> {
        unsafe { self.read().await }
    }

    fn try_read(mut self) -> ReadGuard<'a, T> {
        unsafe { self.try_read() }
    }
}

#[async_trait]
pub trait TopicWriter<T>: Send + Sync {
    fn write(self, value: T);
}

#[async_trait]
pub trait WriterState<T>: Send + Sync {
    unsafe fn write_slice(&mut self, data: &[T])
    where
        T: Clone;
    unsafe fn write(&mut self, value: T);
}

#[async_trait]
impl<T, P: Send + Sync> TopicWriter<T> for P
where
    T: Resource,
    for<'a> P: PipeParam + AsStateMut<'a, Mut = P::State>,
    P::State: WriterState<T>,
{
    fn write(mut self, value: T) {
        unsafe {
            self.as_mut().write(value);
        }
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
            builder.world().get::<DynWriterState<T>>(id).unwrap(),
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
impl<'a, T> TopicWriter<T> for AsyncMutexGuard<'a, Box<dyn WriterState<T>>> {
    fn write(mut self, value: T) {
        unsafe { self.deref_mut().write(value) }
    }
}

pub enum GuardedEvents<'a, T> {
    Guard(SpinRwLockReadGuard<'a, Vec<T>>),
    Slice(&'a [T]),
}

pub struct ReadGuard<'a, T> {
    guard: GuardedEvents<'a, T>,
    range: Range<usize>,
}

impl<'a, T> ReadGuard<'a, T> {
    pub fn new(guard: GuardedEvents<'a, T>, range: Range<usize>) -> Self {
        Self { guard, range }
    }
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match &self.guard {
            GuardedEvents::Guard(x) => &x[self.range.clone()],
            GuardedEvents::Slice(x) => &x[self.range.clone()],
        }
    }
}
