#![allow(non_snake_case)]

use std::{
    io::Read,
    sync::{atomic::AtomicBool, Arc},
};

use async_trait::async_trait;
use bevy_ecs::{
    archetype::Archetype,
    prelude::*,
    system::{SystemMeta, SystemParam, SystemParamFetch, SystemParamState},
};
pub use serde;
use serde::de::DeserializeOwned;
use thiserror::Error;
use zmaxion_core::{bevy_ecs::all_tuples, models::SpawnPipeInner, smallvec::SmallVec};
use zmaxion_utils::prelude::*;

mod async_topic;
mod params_impl;
pub use async_topic::*;
use errors::PipeParamError;
use zmaxion_core::models::{DynPipeDeclaration, PipeDeclaration};
use zmaxion_rt::{AsyncMutex, AsyncMutexGuard, ControlFlow};

mod errors;

pub mod prelude {
    pub use crate::errors::*;
}

#[doc(hidden)]
pub mod __params__ {
    pub use crate::params_impl::*;
}

#[doc(hidden)]
pub mod __async_trait__ {
    pub use async_trait::async_trait;
}

#[derive(Clone, Copy, Debug)]
pub enum TopicParam {
    Reader(Entity),
    Writer(Entity),
}

#[derive(Clone, Copy, Debug)]
pub enum TopicParamKind {
    None,
    Reader,
    Writer,
}

pub struct PipeFactoryArgs {
    pub world: Arc<AsyncMutex<World>>,
    pub config: Arc<SpawnPipeInner>,
    pub should_despawn: Arc<AtomicBool>,
    pub reader_topics: SmallVec<[Entity; 4]>,
    pub writer_topics: SmallVec<[Entity; 4]>,
}

impl PipeFactoryArgs {
    pub fn to_param_builder<Args: DeserializeOwned + Send + Sync, R: Read>(
        &self,
        arg_i: usize,
        topic: Option<TopicParam>,
        args: R,
        defined_kind: TopicParamKind,
    ) -> ParamBuilder<Args> {
        ParamBuilder::new(self, arg_i, topic, args, defined_kind)
    }
}

pub struct ParamBuilder<Args: Send + Sync> {
    world: Arc<AsyncMutex<World>>,
    //    state: Arc<PipelineState>,
    state: Option<(u64, Vec<u8>)>,
    pipe_id: Entity,
    topic: Option<TopicParam>,
    command: Arc<SpawnPipeInner>,
    arg_i: usize,
    args: Option<AnyResult<Args>>,
    defined_param_kind: TopicParamKind,
}

impl<Args: DeserializeOwned + Send + Sync> ParamBuilder<Args> {
    pub fn new<R: Read>(
        factory_args: &PipeFactoryArgs,
        arg_i: usize,
        topic: Option<TopicParam>,
        args: R,
        defined_param_kind: TopicParamKind,
    ) -> Self {
        let args = serde_yaml::from_reader(args);
        ParamBuilder {
            world: factory_args.world.clone(),
            state: None,
            pipe_id: factory_args.config.pipe_id,
            topic,
            command: factory_args.config.clone(),
            arg_i,
            args: Some(args.map_err(|e| e.into())),
            defined_param_kind,
        }
    }
}

impl<Args: DeserializeOwned + Send + Sync> ParamBuilder<Args> {
    pub async fn world<'a>(&'a self) -> AsyncMutexGuard<'a, World> {
        self.world.lock().await
    }

    pub fn entity(&self) -> Entity {
        self.pipe_id
    }

    pub fn command(&self) -> &Arc<SpawnPipeInner> {
        &self.command
    }

    //    pub fn get_state<T: Resource + Default>(&mut self) -> ZionAnyResult<PipeState<T>> {
    //        // state topic needs to be constructed be get reader with 0, also needs to be first
    // when        // spawning
    //        match &self.state {
    //            None => PipeState {
    //                cur_state_generation: 0,
    //                data: T::default(),
    //                writer:
    // TopicWriter::new(self.world.get::<MemTopic<Stateful<T>>>(id).unwrap()),            },
    //            Some(s) => PipeState {
    //                cur_state_generation: s.0,
    //                data: bincode::deserialize(s.1)?,
    //                writer: (),
    //            },
    //        }
    //    }

    pub fn get_topic_id(&self, reader_name: &str) -> AnyResult<Entity, PipeParamError> {
        Ok(match self.defined_param_kind {
            TopicParamKind::None => {
                return Err(PipeParamError::InvalidPipeParam(
                    self.arg_i,
                    reader_name.into(),
                ))
            }
            TopicParamKind::Reader => match self.topic {
                Some(TopicParam::Reader(id)) => id,
                _ => {
                    return Err(PipeParamError::InvalidPipeParam(
                        self.arg_i,
                        reader_name.into(),
                    ))
                }
            },
            TopicParamKind::Writer => match self.topic {
                Some(TopicParam::Writer(id)) => id,
                _ => {
                    return Err(PipeParamError::InvalidPipeParam(
                        self.arg_i,
                        reader_name.into(),
                    ));
                }
            },
        })
    }

    pub fn args(&mut self) -> AnyResult<Args> {
        self.args.take().unwrap()
    }
}

#[async_trait]
pub trait PipeParamState {
    type Args: DeserializeOwned + Send + Sync;
    /// Type of arguments to use when using a constructor
    const KIND: TopicParamKind;
    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
        <Self as PipeParamState>::Args: Sync;
    fn topic_param(&self) -> Option<TopicParam> {
        match Self::KIND {
            TopicParamKind::None => None,
            TopicParamKind::Reader => Some(TopicParam::Reader(Entity::from_raw(u32::MAX))),
            TopicParamKind::Writer => Some(TopicParam::Writer(Entity::from_raw(u32::MAX))),
        }
    }
}

pub trait PipeParam {
    type State;
}

pub trait PipeParamFetch<'s> {
    type Item: PipeParam;
    fn get_param(&'s mut self) -> Self::Item;
}

pub trait PipeSystemParamFetch<'w, 's>: PipeParamState {
    type SystemParam: SystemParam + 's;
    type PipeParam: PipeParam<State = Self> + 's;
    fn get_param(&'s mut self, system_param: &'s mut Self::SystemParam) -> Self::PipeParam;
}

#[async_trait]
impl PipeParamState for () {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::None;

    async fn new(builder: ParamBuilder<()>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        Ok(())
    }

    fn topic_param(&self) -> Option<TopicParam> {
        None
    }
}

#[async_trait]
pub trait PipeParamStateImpl<'s>: Sized + 'static {
    type Source: Component + Clone;
    type Args: DeserializeOwned + Send + Sync + 'static;
    type Param: PipeParam<State = Self>;
    const KIND: TopicParamKind;

    fn new_rw(topic: Self::Source) -> AnyResult<Self>;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self> {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world().await;
        let topic = world.get::<Self::Source>(id).unwrap();
        Self::new_rw(topic.clone())
    }

    #[inline]
    fn should_run(&mut self) -> bool {
        true
    }

    fn topic_param(&self) -> Option<TopicParam> {
        match Self::KIND {
            TopicParamKind::None => None,
            TopicParamKind::Reader => Some(TopicParam::Reader(Entity::from_raw(u32::MAX))),
            TopicParamKind::Writer => Some(TopicParam::Writer(Entity::from_raw(u32::MAX))),
        }
    }

    fn get_param(&'s mut self) -> Self::Param;

    /// Called before the pipe gets executed
    fn pipe_executing(&mut self) -> ControlFlow {
        ControlFlow::default()
    }
    /// Called after the pipe has executed
    fn pipe_executed<T>(&mut self, result: &AnyResult<T>) -> ControlFlow {
        ControlFlow::default()
    }
}

pub trait PipeObserver {
    /// Called before the pipe gets executed
    fn pipe_executing(&mut self) -> ControlFlow {
        ControlFlow::default()
    }
    /// Called after the pipe has executed
    fn pipe_executed<T>(&mut self, result: &AnyResult<T>) -> ControlFlow {
        ControlFlow::default()
    }
}

pub trait IntoParamStateMut<'s>: PipeParam {
    fn into_state_mut(self) -> &'s mut Self::State;
}

pub trait PipeParamImpl {
    type State: PipeParamState + for<'s> PipeParamFetch<'s> + PipeObserver;
}

#[async_trait]
pub trait BuildableParams: PipeParam {
    async fn build(
        args: PipeFactoryArgs,
        pipe_declaration: Arc<DynPipeDeclaration>,
    ) -> Result<Self::State, PipeParamError>;
}
