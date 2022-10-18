use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use bevy_ecs::prelude::*;
use pin_project::pin_project;

#[cfg(not(any(feature = "rt_tokio")))]
compile_error!("one of the features ['rt_tokio'] must be enabled");

#[cfg(feature = "rt_tokio")]
mod rt_tokio;
pub use async_mutex::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};
#[cfg(feature = "rt_tokio")]
pub use tokio;

pub mod prelude {
    #[cfg(feature = "rt_tokio")]
    pub use crate::rt_tokio::*;
    pub use crate::{
        pipe_runtimes::{AsyncRuntimeMarker, BevyRuntimeMarker, SerialRuntimeMarker},
        BlockFutureExt, SpawnFutureExt,
    };
}

use prelude::Handle;
pub use prelude::*;

pub trait AsyncRuntime {
    fn default_multi_thread() -> Self;
    fn default_current_thread() -> Self;
    fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static;
    fn block_on<F>(&self, future: F) -> <F as Future>::Output
    where
        F: Future;
}

pub struct GenericRuntime<T: AsyncRuntime> {
    pub runtime: T,
}

impl<T: AsyncRuntime> GenericRuntime<T> {
    pub fn new(runtime: T) -> Self {
        Self { runtime }
    }

    pub fn default_current_thread() -> Self {
        Self {
            runtime: T::default_current_thread(),
        }
    }

    pub fn default_multi_thread() -> Self {
        Self {
            runtime: T::default_multi_thread(),
        }
    }

    pub fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    pub fn block_on<F>(&self, future: F) -> <F as Future>::Output
    where
        F: Future,
    {
        self.runtime.block_on(future)
    }
}

#[pin_project]
#[derive(Component)]
pub struct GenericTask<T: SpawnedTask>(#[pin] T);

impl<T: SpawnedTask> GenericTask<T> {
    pub fn new(handle: T) -> Self {
        Self(handle)
    }

    pub fn try_poll(&mut self) -> Option<T::Output> {
        self.0.try_poll()
    }

    pub fn poll(&mut self) -> Option<T::InnerOutput> {
        self.0.poll()
    }

    pub fn abort(&self) {
        self.0.abort();
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: SpawnedTask + Unpin> Future for GenericTask<T> {
    type Output = T::InnerOutput;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match SpawnedTask::poll(&mut self.get_mut().0) {
            None => Poll::Pending,
            Some(v) => Poll::Ready(v),
        }
    }
}

pub trait SpawnedTask: Future {
    type InnerOutput;
    /// Check if a future has finished.
    fn try_poll(&mut self) -> Option<Self::Output>;
    /// Check if a future has finished, panics if a thread that is executing a future panics.
    fn poll(&mut self) -> Option<Self::InnerOutput>;
    fn abort(&self);
}

pub trait Notifiable: IsRefAndDefault {
    type Notified;
    fn new() -> Self::Owned;
    fn notified(&self) -> Self::Notified;
    fn notify_one(&self);
    fn notify_waiters(&self);
}

pub struct GenericNotify<T: Notifiable> {
    inner: T::Owned,
}

impl<T: Notifiable> GenericNotify<T>
where
    for<'a> &'a T::Owned: Notifiable,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notified(&self) -> <&<T as IsRefAndDefault>::Owned as Notifiable>::Notified {
        (&self.inner).notified()
    }

    pub fn notify_one(&self) {
        (&self.inner).notify_one()
    }

    pub fn notify_waiters(&self) {
        (&self.inner).notify_waiters()
    }
}

impl<T: Notifiable> Default for GenericNotify<T> {
    fn default() -> Self {
        Self {
            inner: T::Owned::default(),
        }
    }
}

pub trait IsRefAndDefault {
    type Owned: Default;
}

impl<T: Default> IsRefAndDefault for &T {
    type Owned = T;
}

pub struct GenericHandle<T: RuntimeHandle> {
    inner: T,
}

impl<T: RuntimeHandle> GenericHandle<T> {
    pub fn current() -> Self {
        Self {
            inner: T::current(),
        }
    }

    pub fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static,
    {
        self.inner.spawn(future)
    }
}

pub trait RuntimeHandle {
    fn current() -> Self;
    fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static;
}

lazy_static::lazy_static! {
    static ref ASYNC_RUNTIME_HANDLE: Handle = Handle::current();
}

pub fn set_runtime(runtime: &Runtime) {
    runtime.spawn(ASYNC_RUNTIME_HANDLE.spawn(async {}));
}

pub fn spawn<F>(future: F) -> Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    ASYNC_RUNTIME_HANDLE.spawn(future)
}

pub trait SpawnFutureExt: Future {
    fn spawn(self) -> Task<Self::Output>;
}

pub trait BlockFutureExt: Future {
    fn block(self) -> Self::Output;
}

impl<T> BlockFutureExt for T
where
    Self: Future,
{
    fn block(self) -> Self::Output {
        ASYNC_RUNTIME_HANDLE.inner.block_on(self)
    }
}

impl<T> SpawnFutureExt for T
where
    Self: Future + Send + 'static,
    Self::Output: Send + 'static,
{
    fn spawn(self) -> Task<Self::Output> {
        crate::spawn(self)
    }
}

pub mod pipe_runtimes {
    pub use crate::Runtime;

    #[derive(Clone, Default)]
    pub struct SerialRuntimeMarker;
    #[derive(Clone, Default)]
    pub struct BevyRuntimeMarker;
    #[derive(Clone, Default)]
    pub struct AsyncRuntimeMarker;

    impl RuntimeMarker for SerialRuntimeMarker {
        const RUNTIME_KIND: PipeRuntimeKind = PipeRuntimeKind::Sync;
    }
    impl RuntimeMarker for BevyRuntimeMarker {
        const RUNTIME_KIND: PipeRuntimeKind = PipeRuntimeKind::Bevy;
    }
    impl RuntimeMarker for AsyncRuntimeMarker {
        const RUNTIME_KIND: PipeRuntimeKind = PipeRuntimeKind::Async;
    }

    pub trait RuntimeMarker {
        const RUNTIME_KIND: PipeRuntimeKind;
    }

    pub enum PipeRuntimeKind {
        Sync,
        Bevy,
        Async,
    }

    impl Default for PipeRuntimeKind {
        fn default() -> Self {
            Self::Async
        }
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct ControlFlow: u32 {
        const DESPAWN = 1;
        const SKIP = 2;
    }
}

/// Wakes the current task and returns [`Poll::Pending`] once.
///
/// This function is useful when we want to cooperatively give time to the task scheduler. It is
/// generally a good idea to yield inside loops because that way we make sure long-running tasks
/// don't prevent other tasks from running.
///
/// # Examples
///
/// ```
/// use futures_lite::future;
///
/// # spin_on::spin_on(async {
/// future::yield_now().await;
/// # })
/// ```
pub fn yield_now() -> YieldNow {
    YieldNow(false)
}

/// Future for the [`yield_now()`] function.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.0 {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
