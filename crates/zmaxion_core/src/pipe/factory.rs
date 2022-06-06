use std::{io::Read, marker::PhantomData, sync::Arc};
use std::any::Any;

use async_trait::async_trait;
use bevy::ecs::schedule::SystemDescriptor;
use bevy::ecs::system::SystemParam;
use bevy::utils::BoxedFuture;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;

use crate::{
    models::{PipeKind, StaticEstimations},
    pipe::{ParamBuilder, Pipe, SpawnPipeInner, TopicParam},
    prelude::*,
    sync::PrioMutex,
};

#[async_trait]
pub trait PipeFactory: Send + Sync {
    async fn new_pipe(self: Box<Self>, args: PipeFactoryArgs) -> AnyResult<Box<dyn Pipe>>;
    fn dyn_clone(&self) -> Box<dyn PipeFactory>;
}

pub trait Factory2 {
    async fn new_pipe(self: &mut Arc<Self>, args: PipeFactoryArgs) -> AnyResult<Box<dyn Pipe>>;
}

///////////////// winner begin
struct PipeResult {
    pipe: Arc<dyn Pipe>,
    is_async: bool,
    // stores type information neccessary to add a system to the schedule
    pre_baker: Arc<dyn Any>,
    adder: Box<dyn FnMut(&mut dyn Any, &mut StageBuilder)>,
}


//pub trait IntoPipeFactory<Out, Params, Marker>: Sized {
pub trait IntoPipe<Out, Params, Marker>: Sized {
    fn into_pipe(self) -> PipeResult;
}

pub trait PipeBevyFetch {

}

pub trait PipeSystemParamFetch<'s>: PipeParamState + SystemParamState {
    type ParamSystem: SystemParam + 's;
    type ParamPipe: PipeParam<State = Self> + 's;
    fn get_param(&'s mut self, system_param: Self::ParamSystem) -> Self::ParamPipe;
}

fn main() {
    use std::sync::Arc;
    use std::any::Any;
    let a: Arc<dyn Any + Send + Sync> = Arc::new("abc".to_string());
    let b = a.downcast::<String>();
    dbg!(b);
}

pub struct PipeSystemAder {
    storage: Arc<dyn Any>,
    add: fn(Arc<dyn Any>, StageBuilder)
}

pub trait Pipe2 {
    fn add_bevy_system(self: &mut Arc<Self>, builder: StageBuilder);
}
//////////////// winner end

pub struct DynPipe(Arc<dyn Pipe2>);

impl<F, F0, F1> SystemParamFunction<(), (), (F0, F1), ()> for Arc<PipeContainer<F, Params, State, Out>> {
    unsafe fn run(&mut self, input: (), state: &mut bevy_ecs::system::system_param::Fetch, system_meta: &SystemMeta, world: &World, change_tick: u32) -> () {
        todo!()
    }
}

impl<P: Pipe2, Params, Marker> SystemParamFunction<(), (), Params, Marker> for P {
    unsafe fn run(&mut self, input: (), state: (), system_meta: &SystemMeta, world: &World, change_tick: u32) {
        Pipe2::run(self)
    }
}

pub struct PipeContainer<F, Params, State, Out> {
    state: State,
    f: F,
    _t: PhantomData<fn() -> (Params, Out)>,
}

impl<F, F0, F1, Out> PipeContainer<F, (F0, F1), (F0::Fetch, F1::Fetch), Out>
where
    F: FnMut(F0, F1) -> Out,
    F0: SystemParam,
    F1: SystemParam,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _t: Default::default()
        }
    }
}

impl<F, F0, F1, Out> SystemParamFunction<(), (), (), ()> for PipeContainer<F, (F0, F1), Out> {
    unsafe fn run(&mut self, input: (), state: &mut (), system_meta: &SystemMeta, world: &World, change_tick: u32) {
        todo!()
    }
}

fn pipe_factory<T>(args: PipeFactoryArgs) -> BoxedFuture<'static, Arc<dyn Pipe>> {
    #[allow(unused_mut)]
        let mut errors = None;
    $(
    let _out: $out;
    let slice: &[u8] = &[];
    let builder = args.to_param_builder::<(), _>(0, None, slice, ErrorsState::KIND);
    let errors_state: ErrorsState = PipeParamState::new(builder).await.unwrap();
    errors = Some(errors_state);
    )?
    let params: Result<($(<$param as PipeParam>::Fetch,)*), PipeParamError> =
        <paste!([<P $($param)*>])>::build_params::<$($param),*>(args).await;
//                    .map_err(|e| PipeSpawnError::PipeConstructionFailed {
//                        source: e,
//                        pipe_name: args.config.config.name,
//                        pipeline: args.config.pipeline.name,
//                })
    Ok(Box::new(BevyPipe::new(self.f, params?, errors)))
    T::new_pipe(args)
}

pub struct PipeFactoryContainer {
    pub factory: fn(PipeFactoryArgs) -> BoxedFuture<'static, Arc<dyn Pipe>>,
    pub kind: PipeKind,
    pub estimations: Option<StaticEstimations>,
}

pub trait IntoPipeFactory<Out, Params, Marker>: Sized {
    const KIND: PipeKind;
    fn into_pipe_factory(self) -> Box<dyn PipeFactory>;
}

pub struct BevyPipeFactory<Params: SystemParam, F, Out, Marker> {
    pub(super) f: F,
    _params: PhantomData<Params>,
    _out: PhantomData<Out>,
    _marker: PhantomData<Marker>,
}
impl<Params: SystemParam, F, Out, Marker> BevyPipeFactory<Params, F, Out, Marker> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _params: Default::default(),
            _out: Default::default(),
            _marker: Default::default(),
        }
    }
}

impl<Params: SystemParam, F: Clone, Out, Marker> Clone for BevyPipeFactory<Params, F, Out, Marker> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _params: Default::default(),
            _out: Default::default(),
            _marker: Default::default(),
        }
    }
}

pub struct AsyncPipeFactory<Params, F, Fut, Out> {
    pub(super) f: F,
    _params: PhantomData<Params>,
    _fut: PhantomData<Fut>,
    _out: PhantomData<Out>,
}
impl<Params, F, Fut, Out> AsyncPipeFactory<Params, F, Fut, Out> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _params: Default::default(),
            _fut: Default::default(),
            _out: Default::default(),
        }
    }
}

impl<Params, F: Clone, Fut, Out> Clone for AsyncPipeFactory<Params, F, Fut, Out> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _params: Default::default(),
            _fut: Default::default(),
            _out: Default::default(),
        }
    }
}

macro_rules! impl_pipe_factory (
    ($($out: ident)?; $($param: ident),*) => {
        #[async_trait]
        impl<F $(,$out)? $(,$param)*> PipeFactory for BevyPipeFactory<($($param,)*), F, ($(AnyResult<$out>)?), ()>
        where
            for<'a> &'a mut F:
                FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> ($(AnyResult<$out>)?)
                + FnMut($($param),*) -> ($(AnyResult<$out>)?),
            F: Clone
                + Send
                + Sync
                + 'static,
            ($($param,)*): SystemParam<Fetch = ($(<$param as PipeParam>::Fetch,)*)> + PipeParam,
            $($out: Send + Sync + 'static,)?
            $(
                <$param as SystemParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + SystemParamState<Config = Option<<$param as PipeParam>::Fetch>>
                    + PipeObserver,
                $param: SystemParam<Fetch = <$param as PipeParam>::Fetch>
                    + PipeParam
                    + Send
                    + Sync
                    + 'static,
                <$param as PipeParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + PipeParamState
                    + SystemParamFetch<'static, 'static>
                    + Clone,
            )*
        {
            async fn new_pipe(self: Box<Self>, args: PipeFactoryArgs) -> AnyResult<Box<dyn Pipe>> {
                #[allow(unused_mut)]
                let mut errors = None;
                $(
                    let _out: $out;
                    let slice: &[u8] = &[];
                    let builder = args.to_param_builder::<(), _>(0, None, slice, ErrorsState::KIND);
                    let errors_state: ErrorsState = PipeParamState::new(builder).await.unwrap();
                    errors = Some(errors_state);
                )?
                let params: Result<($(<$param as PipeParam>::Fetch,)*), PipeParamError> =
                    <paste!([<P $($param)*>])>::build_params::<$($param),*>(args).await;
//                    .map_err(|e| PipeSpawnError::PipeConstructionFailed {
//                        source: e,
//                        pipe_name: args.config.config.name,
//                        pipeline: args.config.pipeline.name,
//                })
                Ok(Box::new(BevyPipe::new(self.f, params?, errors)))
            }

            fn dyn_clone(&self) -> Box<dyn PipeFactory> {
                Box::new(self.clone())
            }
        }
    };
);

macro_rules! impl_async_pipe_factory (
    ($($out: ident)?; $($param: ident),*) => {
        #[async_trait]
        impl<F, Fut $(,$out)? $(,$param)*> PipeFactory for AsyncPipeFactory<($($param,)*), F, Fut, ($(AnyResult<$out>)?)>
        where
            F: FnMut($($param),*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = ($(AnyResult<$out>)?)> + Send + Sync + 'static,
            $($out: Send + Sync + 'static,)?
            $(
                $param: PipeParam + Send + Sync + 'static,
                <$param as PipeParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + PipeParamState
                    + PipeObserver
                    + Send
                    + Sync,
            )*
        {
            async fn new_pipe(self: Box<Self>, args: PipeFactoryArgs) -> AnyResult<Box<dyn Pipe>> {
                #[allow(unused_mut)]
                let mut errors = None;
                $(
                    let _out: $out;
                    let slice: &[u8] = &[];
                    let builder = args.to_param_builder::<(), _>(0, None, slice, ErrorsState::KIND);
                    let errors_state: ErrorsState = PipeParamState::new(builder).await.unwrap();
                    errors = Some(errors_state);
                )?
                let params: Result<($(<$param as PipeParam>::Fetch,)*), PipeParamError> =
                    <paste!([<P $($param)*>])>::build_params::<$($param),*>(args).await;
//                    .map_err(|e| PipeSpawnError::PipeConstructionFailed {
//                        source: e,
//                        pipe_name: args.config.config.name,
//                        pipeline: args.config.pipeline.name,
//                });
                Ok(Box::new(AsyncPipe::new(self.f, params?, errors)))
            }

            fn dyn_clone(&self) -> Box<dyn PipeFactory> {
                Box::new(self.clone())
            }
        }
    };
);

pub struct AsyncPipeMarker;

macro_rules! impl_into_pipe_factory (
    ($($out: ident)?; $($param: ident),*) => {
        impl<F $(,$out)? $(,$param)*> IntoPipeFactory<($(AnyResult<$out>)?), ($($param,)*), ()> for F
        where
            for<'a> &'a mut F:
                FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> ($(AnyResult<$out>)?)
                + FnMut($($param),*) -> ($(AnyResult<$out>)?),
            F: Clone
                + Send
                + Sync
                + 'static,
            ($($param,)*): SystemParam<Fetch = ($(<$param as PipeParam>::Fetch,)*)> + PipeParam,
            $($out: Send + Sync + 'static,)?
            $(
                <$param as SystemParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + SystemParamState<Config = Option<<$param as PipeParam>::Fetch>>
                    + PipeObserver,
                $param: SystemParam<Fetch = <$param as PipeParam>::Fetch>
                    + PipeParam
                    + Send
                    + Sync
                    + 'static,
                <$param as PipeParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + PipeParamState
                    + SystemParamFetch<'static, 'static>
                    + Clone,
            )*
        {
            const KIND: PipeKind = PipeKind::Bevy;

            fn into_pipe_factory(self) -> Box<dyn PipeFactory> {
                Box::new(BevyPipeFactory::new(self))
            }
        }
    };
);

macro_rules! impl_into_async_pipe_factory (
    ($($out: ident)?; $($param: ident),*) => {
        impl<F, Fut $(,$out)? $(,$param)*> IntoPipeFactory<Fut, ($($param,)*), (AsyncPipeMarker $(,AnyResult<$out>)?)> for F
        where
            F: FnMut($($param),*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = ($(AnyResult<$out>)?)> + Send + Sync + 'static,
            $($out: Send + Sync + 'static,)?
            $(
                $param: PipeParam + Send + Sync + 'static,
                <$param as PipeParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + PipeParamState
                    + PipeObserver
                    + Send
                    + Sync,
            )*
        {
            const KIND: PipeKind = PipeKind::Async;

            fn into_pipe_factory(self) -> Box<dyn PipeFactory> {
                Box::new(AsyncPipeFactory::<($($param,)*), F, Fut, ($(AnyResult<$out>)?)>::new(self))
            }
        }
    };
);

pub(super) use impl_async_pipe_factory;
pub(super) use impl_into_async_pipe_factory;
pub(super) use impl_into_pipe_factory;
pub(super) use impl_pipe_factory;
use zmaxion_utils::prelude::AnyResult;

use crate::pipe::param::{PipeParam, PipeParamState, TopicParamKind};
use crate::prelude::bevy_ecs::system::{SystemMeta, SystemParamState};
