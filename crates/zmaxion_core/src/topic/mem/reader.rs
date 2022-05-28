use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use bevy::ecs::archetype::Archetype;
use bevy::ecs::system::{Resource, SystemMeta, SystemParam, SystemParamFetch, SystemParamState};
use bevy::prelude::*;

use crate::error::{assert_config_provided, AnyResult};
use crate::pipe::param::{
    ParamBuilder, PipeParam, PipeParamFetch, PipeParamState, TopicParam, TopicParamKind,
};
use crate::pipe::PipeObserver;
use crate::resources::GlobalEntity;
use crate::topic::mem::{LocalTopicReadGuard, MemTopic};

#[derive(Component)]
pub struct TopicReaderState<T: Resource> {
    topic: MemTopic<T>,
    n_reader_references: Arc<()>,
    reader_id: usize,
}

impl<T: Resource> TopicReaderState<T> {
    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.topic.read(self.reader_id).await
    }

    pub fn try_read<'a>(&'a self) -> Option<LocalTopicReadGuard<'a, T>> {
        self.topic.try_read(self.reader_id)
    }
}

impl<T: Resource> PipeObserver for TopicReaderState<T> {}

#[async_trait]
impl<T: Resource> PipeParamState for TopicReaderState<T> {
    type Args = ();
    const KIND: TopicParamKind = TopicParamKind::Reader;

    async fn new(builder: ParamBuilder<()>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        Ok(builder.get_reader()?)
    }

    fn configure(&self) -> Option<Self>
    where
        Self: Sized,
    {
        Some((*self).clone())
    }

    fn topic_param(&self) -> Option<TopicParam> {
        Some(TopicParam::Reader(Entity::from_raw(u32::MAX)))
    }
}

unsafe impl<T: Resource> SystemParamState for TopicReaderState<T> {
    type Config = Option<Self>;
    fn init(world: &mut World, system_meta: &mut SystemMeta, config: Self::Config) -> Self {
        assert_config_provided(config)
    }
    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {}
    fn apply(&mut self, world: &mut World) {}
    fn default_config() -> Self::Config {
        None
    }
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for TopicReaderState<T> {
    type Item = TopicReader<'s, T>;
    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        TopicReader { state }
    }
}

impl<T: Resource> Clone for TopicReaderState<T> {
    fn clone(&self) -> Self {
        Self {
            topic: self.topic.clone(),
            n_reader_references: self.n_reader_references.clone(),
            reader_id: self.reader_id,
        }
    }
}

impl<T: Resource> Drop for TopicReaderState<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.n_reader_references) == 1 {
            self.topic.despawn_reader(self.reader_id);
        }
    }
}

impl<T: Resource> From<MemTopic<T>> for TopicReaderState<T> {
    fn from(topic: MemTopic<T>) -> Self {
        Self {
            reader_id: topic.new_reader(),
            topic,
            n_reader_references: Arc::new(()),
        }
    }
}

pub struct TopicReader<'s, T: Resource> {
    state: &'s TopicReaderState<T>,
}

impl<'s, T: Resource> TopicReader<'s, T> {
    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.state.topic.read(self.state.reader_id).await
    }

    pub fn try_read<'a>(&'a self) -> Option<LocalTopicReadGuard<'a, T>> {
        self.state.topic.try_read(self.state.reader_id)
    }
}

impl<'s, T: Resource> SystemParam for TopicReader<'s, T> {
    type Fetch = TopicReaderState<T>;
}

impl<'s, T: Resource> PipeParam for TopicReader<'s, T> {
    type Fetch = TopicReaderState<T>;
}

impl<'s, T: Resource> PipeParamFetch<'s> for ResTopicReaderState<T> {
    type Item = ResTopicReader<'s, T>;

    fn get_param(state: &'s mut Self) -> Self::Item {
        ResTopicReader { state }
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for TopicReaderState<T> {
    type Item = TopicReader<'s, T>;

    fn get_param(state: &'s mut Self) -> Self::Item {
        TopicReader { state }
    }
}

impl<'a, T: Resource> From<&'a TopicReaderState<T>> for TopicReader<'a, T> {
    fn from(state: &'a TopicReaderState<T>) -> Self {
        Self { state }
    }
}

pub struct ResTopicReader<'s, T: Resource> {
    state: &'s ResTopicReaderState<T>,
}

impl<'s, T: Resource> SystemParam for ResTopicReader<'s, T> {
    type Fetch = ResTopicReaderState<T>;
}

impl<'w, 's, T: Resource> PipeParam for ResTopicReader<'s, T> {
    type Fetch = ResTopicReaderState<T>;
}

#[derive(Component)]
pub struct ResTopicReaderState<T: Resource> {
    state: TopicReaderState<T>,
}

impl<T: Resource> ResTopicReaderState<T> {
    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.state.topic.read(self.state.reader_id).await
    }

    pub fn try_read<'a>(&'a self) -> Option<LocalTopicReadGuard<'a, T>> {
        self.state.topic.try_read(self.state.reader_id)
    }
}

unsafe impl<T: Resource> SystemParamState for ResTopicReaderState<T> {
    type Config = Option<Self>;
    fn init(world: &mut World, system_meta: &mut SystemMeta, config: Self::Config) -> Self {
        ResTopicReaderState::from(&*world)
    }
    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {}
    fn default_config() -> Self::Config {
        None
    }
    fn apply(&mut self, world: &mut World) {}
}
impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for ResTopicReaderState<T> {
    type Item = ResTopicReader<'s, T>;
    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        ResTopicReader { state }
    }
}

impl<'s, T: Resource> ResTopicReader<'s, T> {
    pub async fn read<'a>(&'a self) -> LocalTopicReadGuard<'a, T> {
        self.state.read().await
    }

    pub fn try_read<'a>(&'a self) -> Option<LocalTopicReadGuard<'a, T>> {
        self.state.try_read()
    }
}

impl<T: Resource> From<&World> for ResTopicReaderState<T> {
    fn from(world: &World) -> Self {
        let id = world.get_resource::<GlobalEntity>().unwrap().0;
        let topic = world.entity(id).get::<MemTopic<T>>().unwrap();
        Self {
            state: TopicReaderState::from(topic.clone()),
        }
    }
}

#[async_trait]
impl<T: Resource> PipeParamState for ResTopicReaderState<T> {
    type Args = ();
    const KIND: TopicParamKind = TopicParamKind::Reader;

    async fn new(builder: ParamBuilder<()>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        Ok(ResTopicReaderState::from(&*builder.world()))
    }

    fn configure(&self) -> Option<Self>
    where
        Self: Sized,
    {
        Some((*self).clone())
    }

    fn topic_param(&self) -> Option<TopicParam> {
        Some(TopicParam::Reader(Entity::from_raw(u32::MAX)))
    }
}

impl<T: Resource> Clone for ResTopicReaderState<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}
