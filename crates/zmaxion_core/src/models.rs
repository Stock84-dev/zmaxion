use std::{
    borrow::Cow,
    sync::{atomic::AtomicBool, Arc},
};

use bevy_ecs::{prelude::*, system::EntityCommands};
use enum_ordinalize::Ordinalize;
use smallvec::SmallVec;
use zmaxion_rt::pipe_runtimes::{PipeRuntimeKind, RuntimeMarker};
use zmaxion_utils::pool::{PoolArc, PoolItem};

use crate::{
    models::config::{PipeConfig, TopicConfig},
    prelude::*,
    resources::PipelineStateData,
};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct SystemLayoutId(pub u64);
//#[derive(Hash, Eq, PartialEq, Debug, Clone)]
// pub struct TopicId(pub String);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct WorkflowLayoutId(pub u64);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct WorkflowId(pub u64);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct SystemId(pub usize);
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Component)]
pub struct Ids {
    pub layout: u64,
    pub id: u64,
    pub entity: Entity,
}
#[derive(Debug, Clone)]
pub enum Persistance {
    RAM,
    Storage,
}

pub struct SystemLayoutWithId {
    pub layout: PipeLayout,
    pub id: SystemLayoutId,
}

pub struct PipeLayout {
    pub static_estimations: Option<StaticEstimations>,
    pub kind: PipeKind,
}

pub enum SystemDeterminism {
    /// Non deterministic: with same inputs provides different same result
    NonDeterministic,
    /// Deterministic: with same inputs provides same result
    /// Without side effects: Impacts only output topics, must not impact other systems, clients,
    /// databases...
    DeterministicWithoutSideEffects,
}

#[non_exhaustive]
pub enum PipeKind {
    Bevy,
    Async,
}

pub struct StaticEstimations {
    pub ram_usage_bytes: u64,
    pub thread_usage: ThreadUsage,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub network_read_bytes: u64,
    pub network_write_bytes: u64,
}

pub struct ThreadUsage(pub u8);

impl ThreadUsage {
    pub const ALL: ThreadUsage = ThreadUsage(0);
    pub const SINGLE: ThreadUsage = ThreadUsage(1);

    pub fn new(value: u8) -> Self {
        Self { 0: value }
    }
}

impl Default for ThreadUsage {
    fn default() -> Self {
        ThreadUsage::SINGLE
    }
}

pub struct PipelineStateInner {
    state: PipelineStateData,
    updated: AtomicBool,
}

pub mod config {
    use smallvec::SmallVec;

    #[derive(Debug, Clone)]
    pub struct PipeConfig {
        pub name: String,
        pub args: Vec<u8>,
        pub reader_topics: SmallVec<[String; 4]>,
        pub writer_topics: SmallVec<[String; 4]>,
    }

    #[derive(Debug)]
    pub struct TopicConfig {
        pub name: String,
        pub connector: String,
        /// Name of a rust type that is used as a structure for a message
        pub schema: String,
        pub initial_message_set: serde_yaml::Sequence,
        pub n_initial_message_sets: usize,
        pub args: Vec<u8>,
        pub lifetime: TopicLifetime,
        pub access: TopicAccess,
        pub idempotence: Idempotence,
        pub async_support: AsyncSupport,
        pub transactional: TransactionSupport,
    }

    #[derive(Eq, PartialEq, Debug, Clone)]
    #[non_exhaustive]
    pub enum TopicLifetime {
        //    #[default]
        Global,
        Pipeline(String),
    }

    #[derive(Eq, PartialEq, Hash, Debug, Clone, Copy)]
    pub enum TopicAccess {
        Private,
        Public,
    }

    #[derive(Debug)]
    pub enum Idempotence {
        No,
        Yes,
    }

    #[derive(Debug)]
    pub enum AsyncSupport {
        No,
        Yes,
    }

    #[derive(Debug)]
    pub enum TransactionSupport {
        No,
        Yes,
    }
}
pub type PipelineId = i32;

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

#[derive(Debug)]
pub struct SpawnPipeInner {
    pub pipeline_id: Entity,
    pub pipe_id: Entity,
    pub config: Arc<PipeConfig>,
    pub pipeline: Arc<SpawnPipelineInner>,
    pub topic_ids: Arc<Vec<Vec<TopicId<'static>>>>,
    pub pipe_id_rel_to_pipeline: usize,
    pub state_generation: u64,
}
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TopicId<'a> {
    pub name: Cow<'a, str>,
    pub io_mode: IoMode,
    pub pipeline_id: Entity,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum IoMode {
    Read,
    Write,
    ReadWrite,
}

pub struct TopicRef {
    pub id: Entity,
    pub n_references: usize,
}

pub type SchemaTypeName = String;

pub struct TopicSpawnerArgs<'w, 's, 'a> {
    pub commands: EntityCommands<'w, 's, 'a>,
    pub is_reader: bool,
    pub handlers: &'a HashSet<TopicHandler>,
}

pub struct TopicFactory {
    pub factory: fn(&mut TopicSpawnerArgs),
}

#[derive(PartialEq, Eq, Hash, Ordinalize)]
pub enum TopicHandler {
    System,
    Async,
}

#[non_exhaustive]
#[derive(Default, Clone)]
pub struct PipeFeatures<RuntimeMarker> {
    pub runtime: RuntimeMarker,
}

#[non_exhaustive]
#[derive(Default)]
pub struct DynPipeFeatures {
    pub runtime_kind: PipeRuntimeKind,
}

impl DynPipeFeatures {
    pub fn new<T: RuntimeMarker>(features: PipeFeatures<T>) -> Self {
        Self {
            runtime_kind: T::RUNTIME_KIND,
        }
    }
}

impl<T: RuntimeMarker> From<PipeFeatures<T>> for DynPipeFeatures {
    fn from(features: PipeFeatures<T>) -> Self {
        Self::new(features)
    }
}

pub struct PipelineFeatures;

#[derive(Default, Clone)]
pub struct PipeDeclaration<RuntimeMarker> {
    pub param_names: Vec<String>,
    pub features: PipeFeatures<RuntimeMarker>,
}

pub struct DynPipeDeclaration {
    pub param_names: Vec<String>,
    pub features: DynPipeFeatures,
}
