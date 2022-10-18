use std::sync::{atomic::AtomicBool, Arc};

use crate::{
    models::{PipelineStateInner, SpawnPipeInner, TopicId},
    prelude::*,
};

#[derive(Component)]
pub struct Name(pub String);
#[derive(Component)]
pub struct Generation(pub u64);
#[derive(Component)]
pub struct PipeInitializing(pub Arc<SpawnPipeInner>);
#[derive(Component)]
pub struct IsReader;
#[derive(Component)]
pub struct IsWriter;
#[derive(Component)]
pub struct TopicState {
    pub id: TopicId<'static>,
    pub n_references: usize,
}
#[derive(Component)]
pub struct ShouldDespawn(pub Arc<AtomicBool>);
#[derive(Component)]
pub struct Pipeline;
/// An index of a pipe in [`SpawnPipelineInner`].pipes.
#[derive(Component)]
pub struct PipeIdRelToPipeline(pub usize);
#[derive(Component, Clone)]
pub struct PipelineState(Arc<PipelineStateInner>);
/// An Arc that implements Component
#[derive(Component, Deref, DerefMut)]
pub struct Arcc<T>(Arc<T>);

impl<T> Arcc<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(data))
    }
}

impl<T> Clone for Arcc<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}
