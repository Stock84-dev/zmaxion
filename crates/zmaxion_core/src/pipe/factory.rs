use crate::definitions::{PipeKind, StaticEstimations};
use crate::pipe::{ParamBuilder, Pipe, SpawnPipeInner, TopicParam};
use crate::prelude::*;
use crate::sync::PrioMutex;
use async_trait::async_trait;
use bevy::ecs::system::SystemParam;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::io::Read;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct PipeFactoryArgs {
    pub world: Arc<PrioMutex<World>>,
    pub config: Arc<SpawnPipeInner>,
    pub reader_topics: SmallVec<[Entity; 4]>,
    pub writer_topics: SmallVec<[Entity; 4]>,
}

impl PipeFactoryArgs {
    pub fn to_param_builder<Args: DeserializeOwned, R: Read>(
        &self,
        arg_i: usize,
        topic: Option<TopicParam>,
        args: R,
    ) -> ParamBuilder<Args> {
        ParamBuilder::new(self, arg_i, topic, args)
    }
}

#[async_trait]
pub trait PipeFactory: Send + Sync {
    async fn new_pipe(self: Box<Self>, args: PipeFactoryArgs) -> AnyResult<Box<dyn Pipe>>;
    fn dyn_clone(&self) -> Box<dyn PipeFactory>;
}

pub struct PipeFactoryContainer {
    pub factory: Box<dyn PipeFactory>,
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
                    let builder = args.to_param_builder::<(), _>(0, None, slice);
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
                    let builder = args.to_param_builder::<(), _>(0, None, slice);
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
