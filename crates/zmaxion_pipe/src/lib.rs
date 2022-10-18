use std::{any::Any, borrow::Cow, future::Future, marker::PhantomData, sync::Arc};

use bevy_ecs::{
    all_tuples,
    prelude::*,
    schedule::IntoSystemDescriptor,
    system::{SystemParam, SystemParamFetch, SystemState},
};
use ergnomics::prelude::*;
use futures_util::future::{BoxFuture, FutureExt, Pending};
use paste::paste;
use zmaxion_core::models::{DynPipeDeclaration, DynPipeFeatures, PipeFeatures};
use zmaxion_param::{
    __async_trait__::async_trait,
    __params__::*,
    prelude::{ErrorsState, PipeParamError},
    PipeFactoryArgs, PipeObserver, PipeParam, PipeParamFetch, PipeParamState, PipeSystemParamFetch,
    TopicParam, TopicParamKind,
};
use zmaxion_rt::{
    pipe_runtimes::{
        AsyncRuntimeMarker, BevyRuntimeMarker, PipeRuntimeKind, RuntimeMarker, SerialRuntimeMarker,
    },
    prelude::*,
    ControlFlow,
};
use zmaxion_utils::prelude::*;

mod p2;
pub use p2::*;

pub mod components {
    use bevy_ecs::prelude::Entity;
    use zmaxion_core::{prelude::Component, smallvec::SmallVec};
    use zmaxion_rt::Task;
    use zmaxion_utils::prelude::AnyResult;

    use crate::PipeSystemAdder;

    #[derive(Component)]
    pub struct PipeTask(pub Task<AnyResult<Option<Box<dyn PipeSystemAdder>>>>);
    #[derive(Component)]
    pub struct SystemData {
        pub reader_topics: SmallVec<[Entity; 4]>,
        pub writer_topics: SmallVec<[Entity; 4]>,
    }
}

pub mod resources {
    use zmaxion_core::prelude::*;

    use crate::{PipeDefinition, PipeFactory};

    pub struct PipeDefinitions(pub HashMap<String, PipeDefinition>);
}

pub mod prelude {
    pub use zmaxion_core::models::PipeFeatures;
    use zmaxion_rt::{AsyncRuntimeMarker, BevyRuntimeMarker, SerialRuntimeMarker};

    pub fn serial_pipe_features() -> PipeFeatures<SerialRuntimeMarker> {
        PipeFeatures::<SerialRuntimeMarker>::default()
    }
    pub fn bevy_pipe_features() -> PipeFeatures<BevyRuntimeMarker> {
        PipeFeatures::<BevyRuntimeMarker>::default()
    }
    pub fn async_pipe_features() -> PipeFeatures<AsyncRuntimeMarker> {
        PipeFeatures::<AsyncRuntimeMarker>::default()
    }
}

pub struct PipeDefinition {
    pub factory: Box<dyn PipeFactory>,
}

#[derive(Component)]
pub struct Pipe {
    pub adder: Option<Box<dyn PipeSystemAdder>>,
}

pub trait PipeFactory: Send + Sync {
    fn new(
        &mut self,
        args: PipeFactoryArgs,
    ) -> BoxFuture<'static, AnyResult<Option<Box<dyn PipeSystemAdder>>>>;

    fn declaration(&self) -> &DynPipeDeclaration;
}

pub trait PipeSystemAdder: Send + Sync {
    /// SAFETY: system must be removed before adding it again
    unsafe fn add_system(&mut self, builder: &mut SystemStage);
}

impl<T: Send + Sync> PipeSystemAdder for T
where
    T: FnMut(&mut SystemStage),
{
    unsafe fn add_system(&mut self, stage: &mut SystemStage) {
        (self)(stage);
    }
}

// pub trait IntoPipeFactory<Out, Params, Fut, RuntimeMarker>: Sized {
//    fn name(&mut self) -> String {
//        std::any::type_name::<Self>().into()
//    }
//
//    fn into_pipe_factory(self, features: PipeFeatures<RuntimeMarker>) -> Box<dyn PipeFactory>;
//}

struct NonAsyncMarker;
struct ThreadMarker;

macro_rules! impl_into_pipe_factory{
    ($($out:ident)?; $($param:ident),*) => {
        impl<F $(,$out)? $(,$param)*> IntoPipeFactory<($(AnyResult<$out>)?), ($($param,)*), (), BevyRuntimeMarker> for F
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
            fn into_pipe_factory(self, features: PipeFeatures<BevyRuntimeMarker>) -> Box<dyn PipeFactory> {
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
                            ParamGroupBuilder::<($($param,)*)>::build_params(args)
                                .await
                                .map_err(|e| AnyError::from(e))?,
                        );
                        let adder: Box<dyn PipeSystemAdder> = Box::new(move |stage: &mut SystemStage| {
                            let storage = storage.clone();
                            let mut f = f.clone();
                            $(
                                let _out: $out;
                                let errors = errors.clone();
                            )?
                            let system  = move |
                                $(
                                    paste!{mut [<s_ $param>]}: <<$param as PipeParam>::State as PipeSystemParamFetch>::SystemParam,
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
                                    let result;
                                    $(
                                        let paste!{[<s_ $param>]}: &'static mut _ = unsafe {
                                            std::mem::transmute(&mut paste!{[<s_ $param>]})
                                        };
                                    )*
                                    result = ResultConverter::from((f)($(
                                        <<$param as PipeParam>::State as PipeSystemParamFetch>::get_param(
                                            $param,
                                            paste!{[<s_ $param>]},
                                        ),
                                    )*)).0;
                                    let state = unsafe {
                                        &mut *(&*storage as *const _ as *mut ($($param::State,)*))
                                    };
                                    state.pipe_executed(&result);
                                    if let Err(e) = &result {
                                        match e.downcast_ref::<PipeParamError>() {
                                            None => {
                                                $(
                                                    let _out: $out;
                                                    errors.handle(result);
                                                )?
                                            }
                                            Some(&PipeParamError::Despawn) => {
                                                // engine will take care to remove
                                            }
                                        }
                                    }
                                };
                            stage.add_system(system);
                        });
                        Ok(Some(adder))
                    }
                    .boxed()
                });
                factory
            }
        }
    };
}

// trait MyIntoResult {
//    type Ok;
//    fn into_result(self) -> AnyResult<Self::Ok>;
//}
// impl MyIntoResult for () {
//    type Ok = ();
//
//    fn into_result(self) -> AnyResult<Self::Ok> {
//        Ok(())
//    }
//}
// impl<T> MyIntoResult for AnyResult<T> {
//    type Ok = T;
//
//    fn into_result(self) -> AnyResult<Self::Ok> {
//        self
//    }
//}

// struct Runner<'s, Out: MyIntoResult, F, P, S> {
//    state: &'s mut S,
//    f: F,
//    _params: PhantomData<(P, Out)>,
//}
// impl<'s, Out: MyIntoResult, F, P, S> Runner<'s, Out, F, P, S>
// where
//    F: FnMut(P) -> Out,
//    P: PipeParam,
//    S: PipeParamFetch<'s, Item = P> + PipeObserver,
//{
//    fn run(&mut self, state: &mut S) -> Out {
//    }
//
//    async fn run_async(&mut self, state: &mut S) -> Out;
//}

// fn run<'s, Out: MyIntoResult, F, P, S>(f: &mut F, state: &'s mut S, errors: &mut ErrorsState) ->
// Out where
//    F: FnMut(P) -> Out,
//    P: PipeParam,
//    S: PipeParamFetch<'s, Item = P> + PipeObserver,
//{
//    state.pipe_executing();
//    let result = (f)(state.get_param()).into_result();
//    state.pipe_executed(&result);
//    errors.handle(result);
//}

struct ResultConverter<T>(AnyResult<T>);

impl From<()> for ResultConverter<()> {
    fn from(_: ()) -> Self {
        Self(Ok(()))
    }
}

impl<T> From<AnyResult<T>> for ResultConverter<T> {
    fn from(r: AnyResult<T>) -> Self {
        Self(r)
    }
}

use bevy_ecs::{
    prelude::Res,
    world::{World, WorldId},
};

// pub struct PipeExecutor<F, BevyState, State, Params, Fut, Out> {
//    f: F,
//    state: State,
//    params: PhantomData<fn() -> (Params, Out)>,
//}

//#[async_trait]
// pub trait PipeExecutor<Params, Fut, Out> {
//    async fn build(args: PipeFactoryArgs) -> Self;
//    fn execute(&mut self) -> Fut;
//    fn executed(&mut self, result: &AnyResult<Out>);
//}

macro_rules! impl_into_thread_pipe_factory {
    ($($out:ident)?; $($param:ident),*) => {
        impl<F, $($param,)* $($out)?> IntoPipeFactory<($(AnyResult<$out>)?), ($($param,)*), (), SyncRuntimeMarker> for F
        where
            for<'w, 's> F: FnMut($($param),*) -> ($(AnyResult<$out>)?) + Clone + Send + Sync + 'static,
            $($out: Send,)?
            $(
                $param: PipeParam + Send + 'static,
                $param::State: PipeParamFetch<'static, Item = $param> + PipeObserver + Send + Sync,
                <$param as PipeParam>::State: PipeParamState,
            )*
        {
            fn into_pipe_factory(self, features: PipeFeatures<SyncRuntimeMarker>) -> Box<dyn PipeFactory> {
                // using function traits to save compile times because we would need to have multiple traits
                // and implementations
                let factory: Box<dyn PipeFactory> = Box::new(move |args: PipeFactoryArgs| {
                    let mut f = self.clone();
                    let features = features.clone();
                    async move {
                        let world = args.world.clone();
                        let pipeline_id = args.config.pipeline_id;
                        let mut state =
                            ParamGroupBuilder::<($($param,)*)>::build_params(args)
                                .await
                                .map_err(|e| AnyError::from(e))?;
                        let mut world = world.lock(0).unwrap();
                        $(
                            let _out: $out;
                            let mut errors = ErrorsState::new(pipeline_id, &*world);
                        )?
                        spawn::<_, Pending<()>>(&features.runtime_label, &mut *world, Ok(move || {
                            state.pipe_executing();
                            let result;
                            {
                                let ($($param,)*) = &mut state;
                                $(
                                    let paste!{[<s_ $param>]}: &'static mut $param::State = unsafe {
                                        std::mem::transmute($param)
                                    };
                                )*
                                result = ResultConverter::from((f)($(paste!{[<s_ $param>]}.get_param()),*)).0;
                            }
                            state.pipe_executed(&result);
                            if let Err(e) = &result {
                                match e.downcast_ref::<PipeParamError>() {
                                    None => {}
                                    Some(&PipeParamError::Despawn) => {
                                        return ControlFlow::DESPAWN;
                                    }
                                }
                            }
                            $(
                                let _out: $out;
                                errors.handle(result);
                            )?
                            ControlFlow::default()
                        }));
                        Ok(None)
                    }
                    .boxed()
                });
                factory
            }
        }
    };
}

macro_rules! impl_into_async_pipe_factory {
    ($($out:ident)?; $($param:ident),*) => {
        impl<F, $($param,)* Fut $(,$out)?> IntoPipeFactory<($(AnyResult<$out>)?), ($($param,)*), Fut, AsyncRuntimeMarker> for F
        where
            for<'w, 's> F: FnMut($($param),*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = ($(AnyResult<$out>)?)> + Send,
            $($out: Send,)?
            $(
                $param: PipeParam + Send + 'static,
                for<'s> $param::State: PipeParamFetch<'static, Item = $param> + PipeObserver + Send + Sync,
                <$param as PipeParam>::State: PipeParamState,
            )*
        {
            fn into_pipe_factory(self, features: PipeFeatures<AsyncRuntimeMarker>) -> Box<dyn PipeFactory> {
                // using function traits to save compile times because we would need to have multiple traits
                // and implementations
                let factory: Box<dyn PipeFactory> = Box::new(move |args: PipeFactoryArgs| {
                    let mut f = self.clone();
                    let features = features.clone();
                    let config = args.config.clone();
                    async move {
                        let world = args.world.clone();
                        let pipeline_id = args.config.pipeline_id;
                        let mut state =
                            ParamGroupBuilder::<($($param,)*)>::build_params(args)
                                .await
                                .map_err(|e| AnyError::from(e))?;
                        let mut world = world.lock(0).unwrap();
                        $(
                            let _out: $out;
                            let mut errors = ErrorsState::new(pipeline_id, &*world);
                        )?
                        spawn::<fn() -> ControlFlow, _>(&features.runtime_label, &mut *world, Err(async move {
                            loop {
                                state.pipe_executing();
                                let result;
                                let ($($param,)*) = &mut state;
                                $(
                                    let paste!{[<s_ $param>]}: &'static mut $param::State = unsafe {
                                        std::mem::transmute($param)
                                    };
                                )*
                                result = ResultConverter::from((f)($(paste!{[<s_ $param>]}.get_param()),*).await).0;
                                state.pipe_executed(&result);
                                if let Err(e) = &result {
                                    match e.downcast_ref::<PipeParamError>() {
                                        None => {}
                                        Some(&PipeParamError::Despawn) => {
                                            break;
                                        }
                                    }
                                }
                                $(
                                    let _out: $out;
                                    errors.handle(result);
                                )?
                            }
                        }
//                    .instrument(trace_span!("pipeline", name = pipeline_name, id = pipeline_id))
//                    .instrument(trace_span!("pipe", name = pipe_name, id = pipe_id))
                        ));
                        Ok(None)
                    }
                    .instrument(trace_span!("pipeline_build", name = config.pipeline.name, id = ?config.pipeline_id))
                    .instrument(trace_span!("pipe_build", name = config.config.name, id = ?config.pipe_id))
                    .spawn()
                    .boxed()
                });
                factory
            }
        }
    };
}

// fn build_errors(args: &PipeFactoryArgs) -> ErrorsState {
//    let world = args.world.lock(0).unwrap();
//    ErrorsState::new(args.config.pipeline_id, &*world)
//}

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
        impl_into_thread_pipe_factory!(Out; $($param),*);
        impl_into_thread_pipe_factory!(; $($param),*);
        impl_into_pipe_factory!(Out; $($param),*);
        impl_into_pipe_factory!(; $($param),*);
    }
}

// all_tuples!(impl_into_pipe_factories, 0, 16, F);
