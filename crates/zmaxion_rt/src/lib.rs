use std::future::Future;

use bevy::prelude::*;
use pin_project::pin_project;

#[cfg(not(any(feature = "rt_tokio")))]
compile_error!("one of the features ['rt_tokio'] must be enabled");

#[cfg(feature = "rt_tokio")]
mod rt_tokio;
#[cfg(feature = "rt_tokio")]
pub use tokio;

#[doc(hidden)]
pub mod prelude {
    #[cfg(feature = "rt_tokio")]
    pub use crate::rt_tokio::*;
    pub use crate::FutureSpawnExt;
}

use prelude::Handle;
pub use prelude::*;

pub trait AsyncRuntime {
    fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static;
}

pub struct GenericRuntime<T: AsyncRuntime> {
    pub runtime: T,
}

impl<T: AsyncRuntime> GenericRuntime<T> {
    pub fn new(runtime: T) -> Self {
        Self { runtime }
    }

    pub fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }
}

impl<T: AsyncRuntime + Default> Default for GenericRuntime<T> {
    fn default() -> Self {
        Self::new(T::default())
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
    fn current() -> Self {
        Self {
            inner: T::current(),
        }
    }

    fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
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
    runtime.spawn(async { ASYNC_RUNTIME_HANDLE.spawn(async {}) });
}

pub fn spawn<F>(future: F) -> Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    ASYNC_RUNTIME_HANDLE.spawn(future)
}

pub trait FutureSpawnExt: Future {
    fn spawn(self) -> Task<Self::Output>;
}

impl<T> FutureSpawnExt for T
where
    Self: Future + Send + 'static,
    Self::Output: Send + 'static,
{
    fn spawn(self) -> Task<Self::Output> {
        crate::spawn(self)
    }
}
