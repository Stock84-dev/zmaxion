use std::{
    fmt::Display,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub use anyhow::{anyhow, bail, ensure, format_err, Chain, Context, Error, Result as AnyResult};
use async_trait::async_trait;
use bevy::ecs::{
    archetype::Archetype,
    system::{
        FunctionSystem, InputMarker, SystemMeta, SystemParam, SystemParamFetch,
        SystemParamFunction, SystemParamState,
    },
};
use thiserror::Error;

use crate::{
    pipe::{
        param::{ParamBuilder, PipeParam, PipeParamFetch, PipeParamState, TopicParamKind},
        PipeConfig,
    },
    pipeline::SpawnPipelineInner,
    prelude::*,
    resources::LogErrorsSynchronously,
    topic::{messages::SpawnTopicInner, MemTopic, TopicWriterState},
};

#[derive(Error, Debug)]
#[error("{0:#?}")]
pub enum ZmaxionError {
    //    #[error("failed to spawn system from {1:#?}, caused by {0:#?}")]
    //    FailedToSpawnSystem(Box<ZionError>, SpawnSystem),
    //
    //    #[error("system already exists")]
    //    SystemAlreadyExists,
    //    #[error("unregistered system id {0:#?}")]
    //    UnknownSystem(SystemLayoutId),
    //    #[error("unregistered topic id {0:#?}")]
    //    UnknownTopicLayout(TopicLayoutId),
    //    #[error(
    //        "system expected TopicKind::{:#?}, config provided TopicKind::{:#?}",
    //        system,
    //        config
    //    )]
    //    TopicReaderLayoutError {
    //        system: TopicKind,
    //        config: TopicKind,
    //    },
    //    #[error("not enough readers provided (provided: {0:})")]
    //    NotEnoughTopicReaders(usize),
    //    #[error(
    //        "system expected TopicKind::{:#?}, config provided TopicKind::{:#?}",
    //        system,
    //        config
    //    )]
    //    TopicWriterLayoutError {
    //        system: TopicKind,
    //        config: TopicKind,
    //    },
    //    #[error("not enough writers provided (provided: {0:})")]
    //    NotEnoughTopicWriters(usize),
    //    #[error("transaction rolled back successfully: {0:#?}")]
    //    TransactionRolledBackSuccessfully(Box<ZionError>),
    //    #[error("failed to spawn topics {0:#?}")]
    //    FailedToSpawnTopics(Box<ZionError>),
    //    #[error("failed to spawn topics {0:#?}")]
    //    FailedToSpawnSystems(Box<ZionError>),
    //    #[error("transaction failed to roll back: {transaction_error:#?}, rollback cause:
    // {cause:#?}")]    TransactionFailedToRollback {
    //        transaction_error: Box<ZionError>,
    //        cause: Box<ZionError>,
    //    },
    //    Critical(Box<ZionError>),
    //    #[error(
    //        "Failed to construct `{pipe_name:}` pipe for `{pipeline:}` pipeline. \n\n Caused
    // by:\n \         {source:}"
    //    )]
    #[error("Not enough topic writers are present in PipelineConfig.")]
    MissingWriterConfig,
    #[error("Not enough topic readers are present in PipelineConfig.")]
    MissingReaderConfig,
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum SpawnError {
    #[error("Failed to spawn `{}` pipeline.\n\nCaused by:\n    {:?}", info.name, source)]
    Pipeline {
        source: anyhow::Error,
        info: Arc<SpawnPipelineInner>,
    },
    #[error("Failed to spawn a topic\n\nCaused by:\n    {source:?}")]
    Topic {
        source: anyhow::Error,
        info: Option<Arc<SpawnTopicInner>>,
    },
    #[error("Failed to spawn `{}` pipeline.\n\nCaused by:\n    {:?}", info.name, source)]
    Pipe {
        source: anyhow::Error,
        info: Arc<PipeConfig>,
    },
}

#[derive(Error, Debug)]
pub enum TopicSpawnError {
    #[error("Topic `{0:}` is not registered")]
    UnregisteredTopic(String),
    #[error("TopicConfig `{0:}` is not present in PipelineConfig.")]
    MissingTopicConfig(String),
}

#[derive(Error, Debug)]
pub enum PipeSpawnError {
    #[error("Pipe `{0:}` is not registered")]
    UnregisteredPipe(String),
    #[error("Failed to construct `{pipe_name}` pipe for `{pipeline}` pipeline.")]
    PipeConstructionFailed {
        #[source]
        source: anyhow::Error,
        pipe_name: String,
        pipeline: String,
    },
}
#[derive(Error, Debug)]
#[error("{0:}")]
pub enum PipeParamError {
    #[error("Invalid pipe parameter at position {0:} (zero-indexed), expected `{1:}`.")]
    InvalidPipeParam(usize, String),
    Other(anyhow::Error),
}

//#[derive(Error, Debug)]
// pub enum KafkaError {
//    #[error("Failed to deserialize Kafka message (MessagePart::{part:?})")]
//    Deserialization { part: MessagePart, message: Vec<u8> },
//    #[error("Failed to receive message from Kafka")]
//    ConsumerProtocol(rdkafka::error::KafkaError),
//}

#[derive(Debug)]
pub enum MessagePart {
    Key,
    Payload,
    Header,
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum SystemError {
    #[error("system needs to shutdown")]
    ShutdownRequested,
}

pub struct ErrorEvent {
    pub error: anyhow::Error,
    pub pipeline_id: Entity,
}

pub struct Errors<'s> {
    state: &'s ErrorsState,
}

impl<'s> Errors<'s> {
    pub fn handle<T, E>(&self, result: Result<T, E>) -> Option<T>
    where
        E: Into<anyhow::Error> + Send + Sync + 'static,
    {
        self.state.handle(result)
    }

    pub fn handle_with<T, E>(&self, result: Result<T, E>, pipeline_id: Entity) -> Option<T>
    where
        E: Into<anyhow::Error> + Send + Sync + 'static,
    {
        self.state.handle_with(result, pipeline_id)
    }

    pub fn handle_context<T, E, C>(&self, result: Result<T, E>, context: C) -> Option<T>
    where
        E: Into<anyhow::Error> + Send + Sync + 'static,
        Result<T, E>: anyhow::Context<T, E>,
        C: Display + Send + Sync + 'static,
    {
        self.state.handle_context(result, context)
    }
}

impl<'s> From<&'s ErrorsState> for Errors<'s> {
    fn from(state: &'s ErrorsState) -> Self {
        Self { state }
    }
}

impl<'s> SystemParam for Errors<'s> {
    type Fetch = ErrorsState;
}

impl<'s> PipeParam for Errors<'s> {
    type Fetch = ErrorsState;
}

impl<'s> PipeParamFetch<'s> for ErrorsState {
    type Item = Errors<'s>;

    fn get_param(state: &'s mut Self) -> Self::Item {
        Errors { state }
    }
}

#[derive(Clone, Component)]
pub struct ErrorsState {
    pipeline_id: Entity,
    log: Arc<AtomicBool>,
    writer: TopicWriterState<ErrorEvent>,
}

impl ErrorsState {
    pub fn new(pipeline_id: Entity, world: &World) -> Self {
        let topic = world
            .entity(pipeline_id)
            .get::<MemTopic<ErrorEvent>>()
            .unwrap();
        Self {
            pipeline_id,
            log: world
                .get_resource::<LogErrorsSynchronously>()
                .unwrap()
                .0
                .clone(),
            writer: TopicWriterState::from(topic.clone()),
        }
    }

    pub fn handle<T, E>(&self, result: Result<T, E>) -> Option<T>
    where
        E: Into<anyhow::Error> + Send + Sync + 'static,
    {
        match result {
            Ok(x) => Some(x),
            Err(e) => {
                self.write(self.pipeline_id, e.into());
                None
            }
        }
    }

    fn write(&self, pipeline_id: Entity, e: anyhow::Error) {
        if self.log.load(Ordering::SeqCst) {
            error!("{:?}", e);
        }
        self.writer.write(ErrorEvent {
            error: e,
            pipeline_id,
        });
    }

    pub fn handle_with<T, E>(&self, result: Result<T, E>, pipeline_id: Entity) -> Option<T>
    where
        E: Into<anyhow::Error> + Send + Sync + 'static,
    {
        match result {
            Ok(x) => Some(x),
            Err(e) => {
                self.write(pipeline_id, e.into());
                None
            }
        }
    }

    pub fn handle_context<T, E, C>(&self, result: Result<T, E>, context: C) -> Option<T>
    where
        E: Into<anyhow::Error> + Send + Sync + 'static,
        Result<T, E>: Context<T, E>,
        C: Display + Send + Sync + 'static,
    {
        match result.context(context) {
            Ok(x) => Some(x),
            Err(e) => {
                self.write(self.pipeline_id, e);
                None
            }
        }
    }
}

unsafe impl SystemParamState for ErrorsState {
    type Config = Option<Self>;

    fn init(world: &mut World, system_meta: &mut SystemMeta, config: Self::Config) -> Self {
        match config {
            None => {
                let id = world.get_resource::<GlobalEntity>().unwrap().0;
                Self::new(id, world)
            }
            Some(config) => config,
        }
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
    }

    fn apply(&mut self, world: &mut World) {
    }

    fn default_config() -> Self::Config {
        None
    }
}
impl<'w, 's> SystemParamFetch<'w, 's> for ErrorsState {
    type Item = Errors<'s>;

    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        Errors { state }
    }
}

#[async_trait]
impl PipeParamState for ErrorsState {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::None;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.command().pipeline_id;
        Ok(Self::new(id, &*builder.world()))
    }

    fn configure(&self) -> Option<Self>
    where
        Self: Sized,
    {
        Some(self.clone())
    }
}

// impl<'w, 's> From<&World> for Errors<'w, 's> {
//    fn from(world: &World) -> Self {
//        let id = world.get_resource::<GlobalEntity>().unwrap().0;
//        let topic = match world.entity(id).get::<MemTopic<ErrorEvent>>() {
//            None => panic!("Topic `{}` not registered.", ErrorEvent::id_name()),
//            Some(x) => x,
//        };
//        let writer = TopicWriter::from(topic.clone());
//        Errors {
//            errors: writer,
//            pipeline_id: id,
//            _w: Default::default(),
//            _s: Default::default(),
//        }
//    }
//}

// impl<'w, 's> Clone for Errors<'w, 's> {
//    fn clone(&self) -> Errors<'w, 's> {
//        Errors {
//            errors: self.errors.clone(),
//            pipeline_id: self.pipeline_id,
//            _w: Default::default(),
//            _s: Default::default(),
//        }
//    }
//}

pub fn handle_errors<Out>(In(result): In<AnyResult<Out>>, errors: Errors) {
    errors.handle(result);
}

pub(crate) fn assert_config_provided<T: 'static>(config: Option<T>) -> T {
    match config {
        None => {
            let type_name = T::type_name();
            panic!(
                "{} must be initialized with my_system.config(...)",
                type_name
            )
        }
        Some(config) => config,
    }
}

pub trait OptionIntoResultExt {
    type Nullable;
    fn some(self) -> Result<Self::Nullable, crate::error::Error>;
}

impl<T> OptionIntoResultExt for Option<T> {
    type Nullable = T;

    fn some(self) -> Result<Self::Nullable, crate::error::Error> {
        self.ok_or(anyhow::anyhow!(
            "called `Option::unwrap()` on a `None` value"
        ))
    }
}
