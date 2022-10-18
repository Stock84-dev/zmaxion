use std::sync::atomic::AtomicBool;

use bevy_ecs::prelude::*;
use zmaxion_rt::AsyncMutex;
use zmaxion_utils::prelude::*;

use crate::{
    components::PipeIdRelToPipeline,
    models::{SchemaTypeName, TopicFactory, TopicHandler, TopicId, TopicRef},
    prelude::*,
};

pub struct GlobalEntity(pub Entity);
pub struct LoadedConnectors(pub HashSet<String>);
pub struct LogErrorsSynchronously(pub Arc<AtomicBool>);
pub struct Topics(pub HashMap<TopicId<'static>, TopicRef>);
pub struct TopicFactories(pub HashMap<SchemaTypeName, TopicFactory>);
pub struct PipeNameToEntity(pub HashMap<String, Entity>);
pub struct PipelineStateData(pub HashMap<PipeIdRelToPipeline, Mutex<Vec<u8>>>);
pub struct SerializedPipelineState(Vec<u8>);
pub struct WorldArc(pub Arc<AsyncMutex<World>>);
pub struct Reschedule;
pub struct Exit;
pub struct TopicHandlers(pub HashSet<TopicHandler>);
