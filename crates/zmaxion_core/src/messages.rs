use std::sync::Arc;

use bevy_ecs::prelude::Entity;

use crate::models::{config::TopicConfig, SpawnPipeInner, SpawnPipelineInner, TopicId};

pub struct SpawnPipe(pub Arc<SpawnPipeInner>);
pub struct PipeSpawning(pub Arc<SpawnPipeInner>);
pub struct PipeSpawned(pub Arc<SpawnPipeInner>);
pub struct AddSystem {
    pub entity: Entity,
}
pub struct DespawnPipe {
    pub entity: Entity,
}
pub struct LoadPipeState {
    pub state_generation: i64,
    pub state_name: String,
    pub inner: Arc<SpawnPipeInner>,
}
#[derive(Clone)]
pub struct PipeStateLoaded {
    pub inner: Arc<SpawnPipeInner>,
    pub state: Option<Vec<u8>>,
}
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
pub struct SpawnTopics {
    pub pipeline: Arc<SpawnPipelineInner>,
    pub pipeline_id: Entity,
}
pub struct TopicsSpawning {
    pub topic_ids: Arc<Vec<Vec<TopicId<'static>>>>,
    pub pipeline: Arc<SpawnPipelineInner>,
    pub pipeline_id: Entity,
}
pub struct TopicsSpawned {
    pub topic_ids: Arc<Vec<Vec<TopicId<'static>>>>,
    pub pipeline: Arc<SpawnPipelineInner>,
    pub pipeline_id: Entity,
}
#[derive(Debug)]
pub struct SpawnTopicInner {
    pub pipeline: Arc<SpawnPipelineInner>,
    pub pipeline_id: Entity,
    pub config: Arc<TopicConfig>,
    pub is_reader: bool,
}
pub struct TopicSpawning {
    pub topic_id: TopicId<'static>,
    pub id: Entity,
    pub data: Arc<SpawnTopicInner>,
}
pub struct TopicSpawned {
    pub id: Entity,
    pub data: Arc<SpawnTopicInner>,
}
pub struct DespawnTopic {
    pub id: TopicId<'static>,
}
