use std::sync::atomic::AtomicBool;

use bevy_ecs::prelude::*;
use zmaxion_utils::prelude::*;

use crate::{
    components::PipeIdRelToPipeline,
    models::{SchemaTypeName, TopicDefinition, TopicId, TopicRef},
    prelude::*,
};

#[derive(Deref, new)]
pub struct GlobalEntity(Entity);
pub struct LoadedConnectors(pub HashSet<String>);
pub struct LogErrorsSynchronously(pub Arc<AtomicBool>);
pub struct Topics(pub HashMap<TopicId<'static>, TopicRef>);
pub struct TopicDefinitions(pub HashMap<SchemaTypeName, TopicDefinition>);
pub struct PipeNameToEntity(pub HashMap<String, Entity>);
pub struct PipelineStateData(pub HashMap<PipeIdRelToPipeline, Mutex<Vec<u8>>>);
pub struct SerializedPipelineState(Vec<u8>);
pub struct WorldArc(pub Arc<PrioMutex<World>>);
pub struct Reschedule;
pub struct Exit;
