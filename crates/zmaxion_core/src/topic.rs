pub use mem::{
    MemTopic, ResTopicReaderState, ResTopicWriterState, TopicReaderState, TopicWriterState,
};

use std::borrow::Cow;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

mod mem;

pub mod components {
    pub use super::mem::{ResTopicReader, ResTopicWriter, TopicReader, TopicWriter};

    use crate::topic::TopicId;
    use bevy::prelude::*;

    #[derive(Component)]
    pub struct IsReader;
    #[derive(Component)]
    pub struct IsWriter;
    #[derive(Component)]
    pub struct TopicState {
        pub id: TopicId<'static>,
        pub n_references: usize,
    }
}

pub mod messages {
    use crate::definitions::TopicConfig;
    use crate::pipeline::SpawnPipelineInner;
    use crate::topic::TopicId;
    use bevy::prelude::Entity;
    use std::sync::Arc;

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
}

pub mod resources {
    use crate::topic::{SchemaTypeName, Topic, TopicDefinition, TopicId};
    use bevy::utils::HashMap;

    pub struct Topics(pub HashMap<TopicId<'static>, Topic>);
    pub struct TopicDefinitions(pub HashMap<SchemaTypeName, TopicDefinition>);
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum IoMode {
    Read,
    Write,
    ReadWrite,
}

pub struct Topic {
    pub id: Entity,
    pub n_references: usize,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TopicId<'a> {
    pub name: Cow<'a, str>,
    pub io_mode: IoMode,
    pub pipeline_id: Entity,
}

pub type SchemaTypeName = String;

pub struct TopicSpawnerArgs<'w, 's, 'a> {
    pub commands: EntityCommands<'w, 's, 'a>,
    pub is_reader: bool,
}

pub struct TopicDefinition {
    pub spawner: fn(&mut TopicSpawnerArgs),
}
