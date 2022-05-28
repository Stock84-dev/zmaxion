use std::os::linux::raw::stat;
use std::sync::Arc;

use bevy::prelude::*;
use smallvec::SmallVec;

use crate::definitions::{
    AsyncSupport, Idempotence, TopicAccess, TopicConfig, TopicLifetime, TransactionSupport,
};
use crate::pipe::PipeConfig;
use crate::topic::TopicId;

pub type PipelineId = i32;

pub mod components {
    use crate::prelude::*;

    #[derive(Component)]
    pub struct Pipeline;
    /// An index of a pipe in [`SpawnPipelineInner`].pipes.
    #[derive(Component)]
    pub struct PipeIdRelToPipeline(pub usize);
}

pub mod messages {
    use crate::pipeline::SpawnPipelineInner;
    use crate::prelude::*;
    use std::sync::Arc;

    pub struct SpawnPipeline(pub Arc<SpawnPipelineInner>);
    pub struct PipelineSpawning {
        pub id: Entity,
        pub data: Arc<SpawnPipelineInner>,
    }
    pub struct PipelineSpawned {
        pub id: Entity,
        pub data: Arc<SpawnPipelineInner>,
    }
    pub struct DespawnPipeline {
        pub id: Entity,
    }
}

#[derive(Debug, Clone)]
pub struct SpawnPipelineInner {
    pub id: PipelineId,
    pub name: String,
    pub topics: Vec<Arc<TopicConfig>>,
    pub pipes: Vec<Arc<PipeConfig>>,
    pub args: serde_yaml::Mapping,
    pub state_generation: u64,
    pub state_connector: String,
}
