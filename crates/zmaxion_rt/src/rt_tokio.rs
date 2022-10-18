use std::{future::Future, time::Duration};

use futures_lite::future;

use crate::{AsyncRuntime, Notifiable, RuntimeHandle, SpawnedTask};

pub type Runtime = crate::GenericRuntime<TokioRuntime>;
pub type Handle = crate::GenericHandle<tokio::runtime::Handle>;
pub type Task<T> = crate::GenericTask<tokio::task::JoinHandle<T>>;
pub type JoinHandle<T> = tokio::task::JoinHandle<T>;
pub type Notify = crate::GenericNotify<&'static tokio::sync::Notify>;

use tokio::sync::broadcast::Sender;

pub fn sleep(duration: Duration) -> impl Future {
    tokio::time::sleep(duration)
}

pub struct TokioRuntime(pub tokio::runtime::Runtime);

impl Default for TokioRuntime {
    fn default() -> Self {
        Self(tokio::runtime::Runtime::new().unwrap())
    }
}

impl AsyncRuntime for TokioRuntime {
    fn default_multi_thread() -> Self {
        Self(
            tokio::runtime::Builder::new_multi_thread()
                .thread_name("zmaxion_realtime_runtime")
                .enable_all()
                .build()
                .unwrap(),
        )
    }

    fn default_current_thread() -> Self {
        Self(
            tokio::runtime::Builder::new_current_thread()
                .thread_name("zmaxion_worker_runtime")
                .enable_all()
                .build()
                .unwrap(),
        )
    }

    fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static,
    {
        Task::new(tokio::runtime::Runtime::spawn(&self.0, future))
    }

    fn block_on<F>(&self, future: F) -> <F as Future>::Output
    where
        F: Future,
    {
        tokio::runtime::Runtime::block_on(&self.0, future)
    }
}

impl RuntimeHandle for tokio::runtime::Handle {
    fn current() -> Self {
        tokio::runtime::Handle::current()
    }

    fn spawn<F>(&self, future: F) -> Task<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        <F as Future>::Output: Send + 'static,
    {
        Task::new(tokio::runtime::Handle::spawn(self, future))
    }
}

impl<T> SpawnedTask for tokio::task::JoinHandle<T> {
    type InnerOutput = T;

    fn try_poll(&mut self) -> Option<Self::Output> {
        future::block_on(future::poll_once(self))
    }

    fn poll(&mut self) -> Option<Self::InnerOutput> {
        future::block_on(future::poll_once(self))?.ok()
    }

    fn abort(&self) {
        self.abort()
    }
}

impl<'a> Notifiable for &'a tokio::sync::Notify {
    type Notified = tokio::sync::futures::Notified<'a>;

    fn new() -> Self::Owned {
        tokio::sync::Notify::new()
    }

    fn notified(&self) -> Self::Notified {
        tokio::sync::Notify::notified(self)
    }

    fn notify_one(&self) {
        tokio::sync::Notify::notify_one(self)
    }

    fn notify_waiters(&self) {
        tokio::sync::Notify::notify_waiters(self)
    }
}

impl From<tokio::runtime::Runtime> for Runtime {
    fn from(rt: tokio::runtime::Runtime) -> Self {
        Runtime::new(TokioRuntime(rt))
    }
}
