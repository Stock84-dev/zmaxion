use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use bevy::{
    ecs::{
        archetype::Archetype,
        system::{Resource, SystemMeta, SystemParam, SystemParamFetch, SystemParamState},
    },
    prelude::*,
};

use crate::{
    error::{assert_config_provided, AnyResult},
    pipe::{
        param::{
            ParamBuilder, PipeParam, PipeParamFetch, PipeParamState, TopicParam, TopicParamKind,
        },
        PipeObserver,
    },
    prelude::GlobalEntity,
    topic::mem::MemTopic,
};

#[derive(Component)]
pub struct TopicWriterState<T: Resource> {
    topic: MemTopic<T>,
    n_writer_references: Arc<()>,
}

impl<T: Resource> TopicWriterState<T> {
    pub fn write(&self, event: T) {
        self.topic.write(event);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        self.topic.write_all(items);
    }
}

impl<T: Resource> PipeObserver for TopicWriterState<T> {
}

#[async_trait]
impl<T: Resource> PipeParamState for TopicWriterState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Writer;

    async fn new(builder: ParamBuilder<()>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn configure(&self) -> Option<Self>
    where
        Self: Sized,
    {
        Some(self.clone())
    }

    fn topic_param(&self) -> Option<TopicParam> {
        Some(TopicParam::Writer(Entity::from_raw(u32::MAX)))
    }
}

unsafe impl<T: Resource> SystemParamState for TopicWriterState<T> {
    type Config = Option<Self>;

    fn init(world: &mut World, system_meta: &mut SystemMeta, config: Self::Config) -> Self {
        assert_config_provided(config)
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
    }

    fn apply(&mut self, world: &mut World) {
    }

    fn default_config() -> Self::Config {
        None
    }
}
impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for TopicWriterState<T> {
    type Item = TopicWriter<'s, T>;

    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        TopicWriter { state }
    }
}

impl<T: Resource> Clone for TopicWriterState<T> {
    fn clone(&self) -> Self {
        Self {
            topic: self.topic.clone(),
            n_writer_references: self.n_writer_references.clone(),
        }
    }
}

impl<T: Resource> Drop for TopicWriterState<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.n_writer_references) == 1 {
            self.topic.despawn_writer();
        }
    }
}

impl<T: Resource> From<MemTopic<T>> for TopicWriterState<T> {
    fn from(topic: MemTopic<T>) -> Self {
        topic.register_writer();
        Self {
            topic,
            n_writer_references: Arc::new(()),
        }
    }
}

pub struct TopicWriter<'s, T: Resource> {
    state: &'s TopicWriterState<T>,
}

impl<'s, T: Resource> TopicWriter<'s, T> {
    pub fn write(&self, item: T) {
        self.state.topic.write(item);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        self.state.topic.write_all(items);
    }
}

impl<'s, T: Resource> SystemParam for TopicWriter<'s, T> {
    type Fetch = TopicWriterState<T>;
}

impl<'s, T: Resource> PipeParam for TopicWriter<'s, T> {
    type Fetch = TopicWriterState<T>;
}

#[derive(Component)]
pub struct ResTopicWriter<'s, T: Resource> {
    state: &'s ResTopicWriterState<T>,
}

impl<'s, T: Resource> AsRef<ResTopicWriterState<T>> for ResTopicWriter<'s, T> {
    fn as_ref(&self) -> &ResTopicWriterState<T> {
        self.state
    }
}

impl<'s, T: Resource> SystemParam for ResTopicWriter<'s, T> {
    type Fetch = ResTopicWriterState<T>;
}

pub struct ResTopicWriterState<T: Resource> {
    state: TopicWriterState<T>,
}

impl<T: Resource> ResTopicWriterState<T> {
    pub fn write(&self, event: T) {
        self.state.topic.write(event);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        self.state.topic.write_all(items);
    }
}

impl<'s, T: Resource> PipeParamFetch<'s> for ResTopicWriterState<T> {
    type Item = ResTopicWriter<'s, T>;

    fn get_param(state: &'s mut Self) -> Self::Item {
        ResTopicWriter { state }
    }
}

impl<T: Resource> Clone for ResTopicWriterState<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

unsafe impl<T: Resource> SystemParamState for ResTopicWriterState<T> {
    type Config = Option<Self>;

    fn init(world: &mut World, system_meta: &mut SystemMeta, config: Self::Config) -> Self {
        ResTopicWriterState::from(&*world)
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
    }

    fn default_config() -> Self::Config {
        None
    }

    fn apply(&mut self, world: &mut World) {
        self.state.apply(world)
    }
}
impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for ResTopicWriterState<T> {
    type Item = ResTopicWriter<'s, T>;

    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        ResTopicWriter { state }
    }
}

impl<T: Resource> From<&World> for ResTopicWriterState<T> {
    fn from(world: &World) -> Self {
        let id = world.get_resource::<GlobalEntity>().unwrap().0;
        let topic = world.entity(id).get::<MemTopic<T>>().unwrap();
        Self {
            state: TopicWriterState::from(topic.clone()),
        }
    }
}

impl<'s, T: Resource> ResTopicWriter<'s, T> {
    pub fn write(&self, event: T) {
        self.state.state.write(event);
    }

    pub fn write_all(&self, items: impl IntoIterator<Item = T>) {
        self.state.state.write_all(items);
    }
}

impl<'s, T: Resource> PipeParam for ResTopicWriter<'s, T> {
    type Fetch = ResTopicWriterState<T>;
}

impl<'s, T: Resource> PipeParamFetch<'s> for TopicWriterState<T> {
    type Item = TopicWriter<'s, T>;

    fn get_param(state: &'s mut Self) -> Self::Item {
        TopicWriter { state }
    }
}

#[async_trait]
impl<T: Resource> PipeParamState for ResTopicWriterState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Writer;

    async fn new(builder: ParamBuilder<()>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        Ok(ResTopicWriterState::from(&*builder.world()))
    }

    fn configure(&self) -> Option<Self>
    where
        Self: Sized,
    {
        Some((*self).clone())
    }

    fn topic_param(&self) -> Option<TopicParam> {
        Some(TopicParam::Writer(Entity::from_raw(u32::MAX)))
    }
}
