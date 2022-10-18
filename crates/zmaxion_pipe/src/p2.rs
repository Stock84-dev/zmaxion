#![allow(non_snake_case)]

use std::{borrow::Cow, marker::PhantomData, sync::Arc};

use bevy_ecs::{all_tuples, prelude::SystemStage};
use ergnomics::OptionExt;
use futures_util::{
    future::{ready, Future, Map, Ready},
    FutureExt,
};
use zmaxion_core::models::{DynPipeDeclaration, DynPipeFeatures, PipeDeclaration, PipeFeatures};
use zmaxion_param::{
    __params__::ParamGroupBuilder,
    prelude::{Errors, ErrorsState, PipeParamError},
    BuildableParams, PipeFactoryArgs, PipeObserver, PipeParam, PipeParamFetch, PipeParamState,
};
use zmaxion_rt::{
    pipe_runtimes::RuntimeMarker, AsyncRuntimeMarker, BevyRuntimeMarker, BlockFutureExt,
    ControlFlow, RuntimeHandle, SerialRuntimeMarker, SpawnFutureExt,
};
use zmaxion_utils::prelude::AnyError;

use crate::{async_trait, AnyResult, BoxFuture, PipeFactory, PipeSystemAdder};

pub trait Pipe<Params, Fut, Out, Marker> {
    type Ret: Future<Output = AnyResult<Out>>;
    fn run(&mut self, params: Params) -> Self::Ret;
}

macro_rules! impl_async_pipe {
    ($($out:ident)?; $($param:ident),*) => {
        impl<F, Fut  $(,$out)? $(,$param)*> Pipe<($($param,)*), Fut, ($($out)?), ($($out,)?)> for F
        where
            F: FnMut($($param),*) -> Fut,
            Fut: Future<Output = ($(AnyResult<$out>)?)>,
        {
            type Ret = Map<Fut, fn(($(AnyResult<$out>)?)) -> AnyResult<($($out)?)>>;

            fn run(&mut self, params: ($($param,)*)) -> Self::Ret {
                fn map<$($out)?>(a: ($(AnyResult<$out>)?)) -> AnyResult<($($out)?)> {
                    a.into_result()
                }
                let ($($param,)*) = params;
                (self)($($param,)*).map(map::<$($out)?>)
            }
        }
    }
}

macro_rules! impl_pipe {
    ($($out:ident)?; $($param:ident),*) => {
        impl<F  $(,$out)? $(,$param)*> Pipe<($($param,)*), NotFutureMarker, ($($out)?), ($($out,)?)> for F
        where
            F: FnMut($($param),*) -> ($(AnyResult<$out>)?),
        {
            type Ret = Ready<AnyResult<($($out)?)>>;

            fn run(&mut self, params: ($($param,)*)) -> Self::Ret {
                let ($($param,)*) = params;
                ready((self)($($param,)*).into_result())
            }
        }
    }
}

trait IntoResult {
    type Value;
    fn into_result(self) -> AnyResult<Self::Value>;
}

impl IntoResult for () {
    type Value = ();

    fn into_result(self) -> AnyResult<Self::Value> {
        Ok(())
    }
}

impl<T> IntoResult for AnyResult<T> {
    type Value = T;

    fn into_result(self) -> AnyResult<Self::Value> {
        self
    }
}

macro_rules! impl_pipes {
    ($($param:ident),*) => {
        impl_pipe!(Out; $($param),*);
        impl_pipe!(; $($param),*);
        impl_async_pipe!(Out; $($param),*);
        impl_async_pipe!(; $($param),*);
    }
}

all_tuples!(impl_pipes, 0, 16, F);

struct Executor<Params: PipeParam, Pipe, Fut, Out, Marker> {
    state: Params::State,
    pipe: Pipe,
    errors: ErrorsState,
    _t: PhantomData<fn() -> (Params, Pipe, Fut, Out, Marker)>,
}
impl<Params, F, Fut, Out, Marker> Executor<Params, F, Fut, Out, Marker>
where
    Params::State: PipeObserver + PipeParamFetch<'static, Item = Params> + 'static,
    Params: PipeParam + BuildableParams,
    F: Pipe<Params, Fut, Out, Marker>,
{
    async fn new(
        pipe: F,
        args: PipeFactoryArgs,
        pipe_declaration: Arc<DynPipeDeclaration>,
    ) -> Result<Self, PipeParamError> {
        let world = args.world.clone();
        let pipeline_id = args.config.pipeline_id;
        let mut world = world.lock().await;
        let mut errors = ErrorsState::new(pipeline_id, &*world);
        drop(world);
        let mut state = Params::build(args, pipe_declaration).await?;
        Ok(Self {
            state,
            pipe,
            errors,
            _t: Default::default(),
        })
    }

    async fn run(&mut self) -> ControlFlow {
        let flow = self.state.pipe_executing();
        if flow.contains(ControlFlow::SKIP) {
            return ControlFlow::SKIP;
        }
        let static_state: &'static mut Params::State =
            unsafe { std::mem::transmute(&mut self.state) };
        let param = <Params::State as PipeParamFetch<'static>>::get_param(static_state);
        let result = self.pipe.run(param).await;
        self.state.pipe_executed(&result);
        if let Err(e) = &result {
            match e.downcast_ref::<PipeParamError>() {
                None => {}
                Some(&PipeParamError::Despawn) => {
                    return ControlFlow::DESPAWN;
                }
                _ => {}
            }
        }
        self.errors.handle(result);
        ControlFlow::default()
    }
}

pub trait IntoPipeFactory<Params, Fut, Out, PipeMarker, RuntimeMarker>: Sized {
    fn name(&mut self) -> String {
        std::any::type_name::<Self>().into()
    }

    fn into_pipe_factory(self, declaration: PipeDeclaration<RuntimeMarker>)
        -> Box<dyn PipeFactory>;
}

struct PipeFactoryContainer<Params, F, Fut, Out, Marker, RuntimeMarker> {
    declaration: Arc<DynPipeDeclaration>,
    f: F,
    _t: PhantomData<fn() -> (Params, F, Fut, Out, Marker, RuntimeMarker)>,
}

impl<Params, F, Fut, Out, Marker> PipeFactory
    for PipeFactoryContainer<Params, F, Fut, Out, Marker, AsyncRuntimeMarker>
where
    F: Pipe<Params, Fut, Out, Marker> + Clone + Send + Sync + 'static,
    Params: PipeParam + BuildableParams + Send + 'static,
    Marker: 'static,
    Out: 'static,
    Fut: 'static,
    Params::State:
        PipeParamState + PipeParamFetch<'static, Item = Params> + PipeObserver + Send + 'static,
    F::Ret: Send,
{
    fn new(
        &mut self,
        args: PipeFactoryArgs,
    ) -> BoxFuture<'static, AnyResult<Option<Box<dyn PipeSystemAdder>>>> {
        let f = self.f.clone();
        let declaration = self.declaration.clone();
        async move {
            let mut executor = Executor::new(f, args, declaration).await?;
            let pipe = async move {
                loop {
                    if executor.run().await == ControlFlow::DESPAWN {
                        break;
                    }
                }
            }
            .spawn();
            Ok(None)
        }
        .boxed()
    }

    fn declaration(&self) -> &DynPipeDeclaration {
        &self.declaration
    }
}

impl<Params, F, Fut, Out, Marker> PipeFactory
    for PipeFactoryContainer<Params, F, Fut, Out, Marker, SerialRuntimeMarker>
where
    F: Pipe<Params, Fut, Out, Marker> + Clone + Send + Sync + 'static,
    Params: PipeParam + BuildableParams + Send + 'static,
    Params::State:
        PipeParamState + PipeParamFetch<'static, Item = Params> + PipeObserver + Send + 'static,
    Marker: 'static,
    Out: 'static,
    Fut: 'static,
    F::Ret: Send,
{
    fn new(
        &mut self,
        args: PipeFactoryArgs,
    ) -> BoxFuture<'static, AnyResult<Option<Box<dyn PipeSystemAdder>>>> {
        let f = self.f.clone();
        let declaration = self.declaration.clone();
        async move {
            let mut executor = Executor::new(f, args, declaration).await?;
            std::thread::spawn(move || {
                async move {
                    loop {
                        if executor.run().await == ControlFlow::DESPAWN {
                            break;
                        }
                    }
                }
                .block();
            });
            Ok(None)
        }
        .boxed()
    }

    fn declaration(&self) -> &DynPipeDeclaration {
        &self.declaration
    }
}

impl<Params, F, Fut, Out, Marker> PipeFactory
    for PipeFactoryContainer<Params, F, Fut, Out, Marker, BevyRuntimeMarker>
where
    F: Pipe<Params, Fut, Out, Marker> + Clone + Send + Sync + Sync + 'static,
    Params: PipeParam + BuildableParams + Send + Sync,
    Params::State: PipeParamState
        + PipeParamFetch<'static, Item = Params>
        + PipeObserver
        + Send
        + Sync
        + 'static,
    F::Ret: Send + Sync,
{
    fn new(
        &mut self,
        args: PipeFactoryArgs,
    ) -> BoxFuture<'static, AnyResult<Option<Box<dyn PipeSystemAdder>>>> {
        let f = self.f.clone();
        let declaration = self.declaration.clone();
        async move {
            let mut executor = Executor::new(f, args, declaration).await?;
            let adder: Box<dyn PipeSystemAdder> = Box::new(move |stage: &mut SystemStage| {
                todo!("Implement bevy runtime");
                //                let system  = move |
                //                $(
                //                    paste!{mut [<s_ $param>]}: <<$param as PipeParam>::State as
                // PipeSystemParamFetch>::SystemParam,                )*
                //                    | {
                //                    // SAFETY: there won't be multiple systems running, system
                // will be                    // removed before it is added again,
                // forwarded to the caller in                    //
                // [`PipeSystemAdder`]                    let state = unsafe {
                //                        &mut *(&*storage as *const _ as *mut ($($param::State,)*))
                //                    };
                //                    state.pipe_executing();
                //                    let ($($param,)*) = state;
                //                    let result;
                //                    $(
                //                    let paste!{[<s_ $param>]}: &'static mut _ = unsafe {
                //                        std::mem::transmute(&mut paste!{[<s_ $param>]})
                //                    };
                //                    )*
                //                        result = ResultConverter::from((f)($(
                //                        <<$param as PipeParam>::State as
                // PipeSystemParamFetch>::get_param(                        $param,
                //                        paste!{[<s_ $param>]},
                //                    ),
                //                    )*)).0;
                //                    let state = unsafe {
                //                        &mut *(&*storage as *const _ as *mut ($($param::State,)*))
                //                    };
                //                    state.pipe_executed(&result);
                //                    if let Err(e) = &result {
                //                        match e.downcast_ref::<PipeParamError>() {
                //                            None => {
                //                                $(
                //                                let _out: $out;
                //                                errors.handle(result);
                //                                )?
                //                            }
                //                            Some(&PipeParamError::Despawn) => {
                //                                // engine will take care to remove
                //                            }
                //                        }
                //                    }
                //                };
                //                stage.add_system(system);
            });
            Ok(Some(adder))
        }
        .boxed()
    }

    fn declaration(&self) -> &DynPipeDeclaration {
        &self.declaration
    }
}

impl<Params, F, Fut, Out, Marker, RM> IntoPipeFactory<Params, Fut, Out, Marker, RM> for F
where
    F: Send + Sync + 'static,
    Marker: 'static,
    Fut: 'static,
    Params: PipeParam + BuildableParams + 'static,
    Out: 'static,
    Params::State: PipeObserver + PipeParamFetch<'static, Item = Params> + Send + 'static,
    F: Pipe<Params, Fut, Out, Marker> + Send + Sync + Clone,
    RM: RuntimeMarker + 'static,
    PipeFactoryContainer<Params, F, Fut, Out, Marker, RM>: PipeFactory,
{
    fn into_pipe_factory(self, declaration: PipeDeclaration<RM>) -> Box<dyn PipeFactory> {
        Box::new(PipeFactoryContainer::<Params, F, Fut, Out, Marker, RM> {
            declaration: Arc::new(DynPipeDeclaration {
                param_names: declaration.param_names,
                features: declaration.features.into(),
            }),
            f: self,
            _t: Default::default(),
        })
    }
}
struct NotFutureMarker;

// fn assert_pipe<P: Pipe<Params, Fut, Out, Marker>, Params, Fut, Out, Marker>(p: P) {
//}
// fn a() {
//    assert_pipe(simple)
//}
// async fn simple() -> AnyResult<()> {
//    Ok(())
//}
