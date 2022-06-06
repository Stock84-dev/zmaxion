use std::{any::Any, future::Future, marker::PhantomData, sync::Arc};

use bevy_ecs::{
    all_tuples,
    prelude::*,
    schedule::IntoSystemDescriptor,
    system::{SystemParam, SystemParamFetch},
};
use futures_util::future::{BoxFuture, FutureExt};
use paste::paste;
use zmaxion_param::{
    PipeFactoryArgs, PipeObserver, PipeParam, PipeParamFetch, PipeParamState, PipeSystemParamFetch,
    __params__::*, prelude::ErrorsState, TopicParam, TopicParamKind,
};
use zmaxion_rt::prelude::*;
use zmaxion_utils::prelude::*;

pub struct StageBuilder;

impl StageBuilder {
    //    pub fn add_system<Params: SystemParam, T: SystemParamFunction<(), (), Params, ()>>(
    //        &mut self,
    //        system: T,
    //    ) {
    //    }
    pub fn add_system<T: IntoSystemDescriptor<Params>, Params>(&mut self, system: T) {
    }
}

pub trait IntoPipeFactory<Out, Params, Fut>: Sized {
    fn into_pipe_factory(self) -> Box<dyn PipeFactory>;
}

pub trait PipeFactory {
    fn new(
        &mut self,
        args: PipeFactoryArgs,
    ) -> BoxFuture<'static, AnyResult<Option<Box<dyn PipeSystemAdder>>>>;
}

impl<T> PipeFactory for T
where
    T: FnMut(PipeFactoryArgs) -> BoxFuture<'static, AnyResult<Option<Box<dyn PipeSystemAdder>>>>,
{
    fn new(
        &mut self,
        args: PipeFactoryArgs,
    ) -> BoxFuture<'static, AnyResult<Option<Box<dyn PipeSystemAdder>>>> {
        (self)(args)
    }
}

pub trait PipeSystemAdder {
    /// SAFETY: system must be removed before adding it again
    unsafe fn add_system(&mut self, builder: &mut StageBuilder);
}

impl<T> PipeSystemAdder for T
where
    T: FnMut(&mut StageBuilder),
{
    unsafe fn add_system(&mut self, builder: &mut StageBuilder) {
        (self)(builder);
    }
}

struct NonAsyncMarker;

macro_rules! impl_into_pipe_factory(
    ($($out:ident)?; $($param:ident),*) => {
        impl<F $(,$out)? $(,$param)*> IntoPipeFactory<($(AnyResult<$out>)?), ($($param,)*), NonAsyncMarker> for F
        where
            for<'w, 's> F:
                FnMut($($param),*) -> ($(AnyResult<$out>)?) + Clone + Send + Sync + 'static,
            $($out: Send,)?
            $(
                $param: PipeParam + 'static,
                for<'w, 's> $param::State: PipeSystemParamFetch<'w, 's, PipeParam = $param> + PipeObserver + Send + Sync,
                for<'w, 's> <<<$param as PipeParam>::State as PipeSystemParamFetch<'static, 'static>>::SystemParam as SystemParam>::Fetch:
                    SystemParamFetch<
                        'w,
                        's,
                        Item = <<$param as PipeParam>::State as PipeSystemParamFetch<'static, 'static>>::SystemParam,
                    >,
            )*
        {
            fn into_pipe_factory(self) -> Box<dyn PipeFactory> {
                // using function traits to save compile times because we would need to have multiple traits
                // and implementations
                let factory: Box<dyn PipeFactory> = Box::new(move |args: PipeFactoryArgs| {
                    let f = self.clone();
                    async move {
                        $(
                            let _out: $out;
                            let mut errors = build_errors(&args);
                        )?
                        let storage = Arc::new(
                            paste!{[<P$($param)*>]::build_params::<$($param),*>(args)}
                                .await
                                .map_err(|e| AnyError::from(e))?,
                        );
                        let adder: Box<dyn PipeSystemAdder> = Box::new(move |builder: &mut StageBuilder| {
                            let storage = storage.clone();
                            let mut f = f.clone();
                            $(
                                let _out: $out;
                                let errors = errors.clone();
                            )?
                            let system  = move |
                                $(
                                    paste!{[<s_ $param>]}: <<$param as PipeParam>::State as PipeSystemParamFetch>::SystemParam,
                                )*
                                | {
                                    // SAFETY: there won't be multiple systems running, system will be
                                    // removed before it is added again, forwarded to the caller in
                                    // [`PipeSystemAdder`]
                                    let state = unsafe {
                                        &mut *(&*storage as *const _ as *mut ($($param::State,)*))
                                    };
                                    state.pipe_executing();
                                    let ($($param,)*) = state;
                                    let result = MyResult::from((f)($(
                                        <<$param as PipeParam>::State as PipeSystemParamFetch>::get_param(
                                            $param,
                                            paste!{[<s_$param>]},
                                        ),
                                    )*)).0;
                                    state.pipe_executed(&result);
                                    $(
                                        let _out: $out;
                                        errors.handle(result);
                                    )?
                                };
                            builder.add_system(system);
                        });
                        Ok(Some(adder))
                    }
                    .boxed()
                });
                factory
            }
        }
    };
);

trait MyIntoResult {
    type Ok;
    fn into_result(self) -> AnyResult<Self::Ok>;
}

impl MyIntoResult for () {
    type Ok = ();

    fn into_result(self) -> AnyResult<Self::Ok> {
        Ok(())
    }
}

impl<T> MyIntoResult for AnyResult<T> {
    type Ok = T;

    fn into_result(self) -> AnyResult<Self::Ok> {
        self
    }
}

struct Runner<'s, Out: MyIntoResult, F, P, S> {
    state: &'s mut S,
    f: F,
    _params: PhantomData<(P, Out)>,
}

impl<'s, Out: MyIntoResult, F, P, S> Runner<'s, Out, F, P, S>
where
    F: FnMut(P) -> Out,
    P: PipeParam,
    S: PipeParamFetch<'s, Item = P> + PipeObserver,
{
    fn run(&mut self, state: &mut S) -> Out {
    }

    async fn run_async(&mut self, state: &mut S) -> Out;
}

fn run<'s, Out: MyIntoResult, F, P, S>(f: &mut F, state: &'s mut S, errors: &mut ErrorsState) -> Out
where
    F: FnMut(P) -> Out,
    P: PipeParam,
    S: PipeParamFetch<'s, Item = P> + PipeObserver,
{
    state.pipe_executing();
    let result = (f)(state.get_param()).into_result();
    state.pipe_executed(&result);
    errors.handle(result);
}

struct MyResult<T>(AnyResult<T>);

impl From<()> for MyResult<()> {
    fn from(_: ()) -> Self {
        Self(Ok(()))
    }
}

impl<T> From<AnyResult<T>> for MyResult<T> {
    fn from(r: AnyResult<T>) -> Self {
        Self(r)
    }
}

macro_rules! impl_into_async_pipe_factory {
    ($($out:ident)?; $($param:ident),*) => {
        impl<F, $($param,)* Fut $(,$out)?> IntoPipeFactory<($(AnyResult<$out>)?), ($($param,)*), Fut> for F
        where
            for<'w, 's> F: FnMut($($param),*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = ($(AnyResult<$out>)?)> + Send,
            $($out: Send,)?
            $(
                $param: PipeParam + Send + 'static,
                for<'s> $param::State: PipeParamFetch<'s, Item = $param> + PipeObserver + Send + Sync,
                <$param as PipeParam>::State: PipeParamState,
            )*
        {
            fn into_pipe_factory(self) -> Box<dyn PipeFactory> {
                // using function traits to save compile times because we would need to have multiple traits
                // and implementations
                let factory: Box<dyn PipeFactory> = Box::new(move |args: PipeFactoryArgs| {
                    let mut f = self.clone();
                    $(
                        let _out: $out;
                        let mut errors: ErrorsState;
                        errors = build_errors(&args);
                    )?
                    async move {
                        let mut state =
                            paste!{[<P$($param)*>]::build_params::<$($param),*>(args)}
                                .await
                                .map_err(|e| AnyError::from(e))?;
                        async move {
                            loop {
                                state.pipe_executing();
                                let result;
                                {
                                    let ($($param,)*) = &mut state;
                                    result = MyResult::from((f)($($param.get_param()),*).await).0;
                                }
                                state.pipe_executed(&result);
                                $(
                                    let _out: $out;
                                    errors.handle(result);
                                )?
                            }
                        }
                        .spawn();
                        Ok(None)
                    }
                    .boxed()
                });
                factory
            }
        }
    };
}

fn build_errors(args: &PipeFactoryArgs) -> ErrorsState {
    let world = args.world.lock(0).unwrap();
    ErrorsState::new(args.config.pipeline_id, &*world)
}

// fn assert_fn_mut<T: Send + Sync + 'static, F0: SystemParam, F1: SystemParam>(t: T) -> T
// where
//    for<'a> &'a mut T: FnMut(F0, F1) -> (),
//    for<'a> &'a mut T:
//        FnMut(<F0::Fetch as SystemParamFetch>::Item, <F1::Fetch as SystemParamFetch>::Item) -> (),
//    for<'a> &'a mut T: FnOnce(F0, F1) -> (),
//{
//    assert_spf::<_, (F0, F1), ()>(t)
//}
// fn assert_spf<T: SystemParamFunction<(), (), Param, Marker>, Param: SystemParam, Marker>(
//    t: T,
//) -> T {
//    t
//}

macro_rules! impl_into_pipe_factories {
    ($($param:ident),*) => {
        impl_into_async_pipe_factory!(Out; $($param),*);
        impl_into_async_pipe_factory!(; $($param),*);
        impl_into_pipe_factory!(Out; $($param),*);
        impl_into_pipe_factory!(; $($param),*);
    }
}

all_tuples!(impl_into_pipe_factories, 0, 16, F);

enum MyFut<F> {
    Ready(i32),
    Fut(F),
}

pub async fn process(val: &mut i32) {
    match f() {
        MyFut::Ready(f) => {
            *val -= f;
        }
        MyFut::Fut(f) => {
            *val += f.await;
        }
    }
}

fn f() -> MyFut<impl Future<Output = i32>> {
    MyFut::Fut(async { 69 })
}
