use std::{
    fmt::Display,
    sync::atomic::{AtomicBool, Ordering},
};

use async_trait::async_trait;
use bevy_ecs::{
    archetype::Archetype,
    prelude::*,
    system::{SystemMeta, SystemParam, SystemParamState},
};
use zmaxion_core::{components::Arcc, prelude::GlobalEntity, resources::LogErrorsSynchronously};
use zmaxion_utils::prelude::*;

use crate::{
    AsyncTopic, AsyncTopicWriterState, ParamBuilder, PipeFactoryArgs, PipeParam, PipeParamFetch,
    PipeParamState, TopicParam, TopicParamKind,
};

pub struct ErrorEvent {
    pub error: AnyError,
    pub pipeline_id: Entity,
}

pub struct Errors<'s> {
    state: &'s ErrorsState,
}

impl<'s> Errors<'s> {
    pub fn handle<T, E>(&self, result: Result<T, E>) -> Option<T>
    where
        E: Into<AnyError> + Send + Sync + 'static,
    {
        self.state.handle(result)
    }

    pub fn handle_with<T, E>(&self, result: Result<T, E>, pipeline_id: Entity) -> Option<T>
    where
        E: Into<AnyError> + Send + Sync + 'static,
    {
        self.state.handle_with(result, pipeline_id)
    }

    pub fn handle_context<T, E, C>(&self, result: Result<T, E>, context: C) -> Option<T>
    where
        E: Into<AnyError> + Send + Sync + 'static,
        Result<T, E>: AnyContext<T, E>,
        C: Display + Send + Sync + 'static,
    {
        self.state.handle_context(result, context)
    }
}

impl ErrorsState {
    pub fn handle<T, E>(&self, result: Result<T, E>) -> Option<T>
    where
        E: Into<AnyError> + Send + Sync + 'static,
    {
        match result {
            Ok(x) => Some(x),
            Err(e) => {
                self.write(self.pipeline_id, e.into());
                None
            }
        }
    }

    pub fn handle_with<T, E>(&self, result: Result<T, E>, pipeline_id: Entity) -> Option<T>
    where
        E: Into<AnyError> + Send + Sync + 'static,
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
        E: Into<AnyError> + Send + Sync + 'static,
        Result<T, E>: AnyContext<T, E>,
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

    fn write(&self, pipeline_id: Entity, e: AnyError) {
        if self.log.load(Ordering::SeqCst) {
            error!("{:?}", e);
        }
        self.writer.write_raw(ErrorEvent {
            error: e,
            pipeline_id,
        });
    }
}

impl<'s> From<&'s ErrorsState> for Errors<'s> {
    fn from(state: &'s ErrorsState) -> Self {
        Self { state }
    }
}

impl<'s> PipeParam for Errors<'s> {
    type State = ErrorsState;
}

impl<'s> PipeParamFetch<'s> for ErrorsState {
    type Item = Errors<'s>;

    fn get_param(&'s mut self) -> Self::Item {
        Errors { state: self }
    }
}

#[derive(Component)]
pub struct ErrorsState {
    pipeline_id: Entity,
    log: Arc<AtomicBool>,
    writer: AsyncTopicWriterState<ErrorEvent>,
}

impl ErrorsState {
    pub fn new(pipeline_id: Entity, world: &World) -> Self {
        let topic = world
            .entity(pipeline_id)
            .get::<Arcc<AsyncTopic<ErrorEvent>>>()
            .unwrap();
        Self {
            pipeline_id,
            log: world
                .get_resource::<LogErrorsSynchronously>()
                .unwrap()
                .0
                .clone(),
            writer: AsyncTopicWriterState::from(Arc::clone(&*topic)),
        }
    }
}

impl Clone for ErrorsState {
    fn clone(&self) -> Self {
        Self {
            pipeline_id: self.pipeline_id,
            log: self.log.clone(),
            writer: AsyncTopicWriterState::from(self.writer.topic().clone()),
        }
    }
}

#[async_trait]
impl PipeParamState for ErrorsState {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Writer;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.command().pipeline_id;
        Ok(Self::new(id, &*builder.world()))
    }
}
