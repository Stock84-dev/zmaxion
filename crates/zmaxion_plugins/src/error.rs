use std::sync::Arc;

use thiserror::Error;
use zmaxion_core::{
    messages::SpawnTopicInner,
    models::{config::PipeConfig, SpawnPipelineInner},
};

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

