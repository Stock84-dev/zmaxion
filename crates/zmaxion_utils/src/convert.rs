use std::{
    convert::Infallible,
    future,
    future::{Future, Ready},
};

use futures_util::FutureExt;

pub struct FutureMarker;
pub struct NotFutureMarker;
pub struct ResultMarker;
pub struct NotResultMarker;

pub trait IntoTryFuture<FutureMarker, ResultMarker> {
    type T;
    type E;
    type Fut: Future<Output = Result<Self::T, Self::E>>;
    fn into_try_future(self) -> Self::Fut;
}

impl<F: Future> IntoTryFuture<FutureMarker, NotResultMarker> for F {
    type E = Infallible;
    type Fut = futures_util::future::Map<F, fn(Self::T) -> Result<Self::T, Infallible>>;
    type T = F::Output;

    fn into_try_future(self) -> Self::Fut {
        fn map<T>(x: T) -> Result<T, Infallible> {
            Ok(x)
        }
        self.map(map::<Self::T>)
    }
}

impl<T, E, F: Future<Output = Result<T, E>>> IntoTryFuture<FutureMarker, ResultMarker> for F {
    type E = E;
    type Fut = Self;
    type T = T;

    fn into_try_future(self) -> Self::Fut {
        self
    }
}

impl<T, E> IntoTryFuture<NotFutureMarker, ResultMarker> for Result<T, E> {
    type E = E;
    type Fut = Ready<Result<T, E>>;
    type T = T;

    fn into_try_future(self) -> Self::Fut {
        future::ready(self)
    }
}

impl<T> IntoTryFuture<NotFutureMarker, NotResultMarker> for T {
    type E = Infallible;
    type Fut = Ready<Result<T, Infallible>>;
    type T = T;

    fn into_try_future(self) -> Self::Fut {
        future::ready(Ok(self))
    }
}
