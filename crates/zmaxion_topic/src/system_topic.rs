use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use bevy_ecs::{
    archetype::Archetype,
    system::{Resource, SystemMeta, SystemParamFetch, SystemParamState},
};
use zmaxion_core::prelude::*;
use zmaxion_param::{
    prelude::*, ParamBuilder, PipeObserver, PipeParam, PipeParamState, PipeSystemParamFetch,
    TopicParamKind,
};
use zmaxion_utils::prelude::*;

use crate::{
    bevy_ecs::system::SystemParam,
    dyn_topic::{GuardedEvents, ReaderState, TopicReader, TopicWriter, WriterState},
    HasTopicFeatures, ReadGuard, Semantics, TopicFeatures,
};

#[derive(Component)]
pub struct SystemTopic<T> {
    read_events: Vec<T>,
    write_events: Vec<T>,
}

impl<T> Default for SystemTopic<T> {
    fn default() -> Self {
        Self {
            read_events: vec![],
            write_events: vec![],
        }
    }
}

impl<T: Resource> HasTopicFeatures for SystemTopic<T> {
    const FEATURES: TopicFeatures = TopicFeatures {
        system_execution: true,
        async_execution: false,
        cursors_cached: false,
        semantics: Semantics::AtMostOnce,
    };

    fn add_system(stage: &mut SystemStage) {
        stage.add_system(Self::update_system);
    }
}

#[derive(SystemParam)]
pub struct BevySystemTopicReader<'w, 's, T: Resource> {
    query: Query<'w, 's, &'static SystemTopic<T>>,
}

#[derive(SystemParam)]
pub struct GlobalSystemReader<'w, 's, T: Resource> {
    query: Query<'w, 's, &'static SystemTopic<T>>,
    global_id: Res<'w, GlobalEntity>,
}

impl<'w: 's, 's, T: Resource> GlobalSystemReader<'w, 's, T> {
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        let topic = self.global_id.get(&self.query);
        let iter = topic.read_events.iter();
        iter
    }

    pub fn read(&self) -> &[T] {
        &self.global_id.get(&self.query).read_events
    }
}

impl<'w: 's, 's, T: Resource> IntoIterator for &'s GlobalSystemReader<'w, 's, T> {
    type IntoIter = std::slice::Iter<'s, T>;
    type Item = &'s T;

    fn into_iter(self) -> Self::IntoIter {
        let topic = self.global_id.get(&self.query);
        let iter = topic.read_events.iter();
        iter
    }
}

pub struct SystemTopicReader<'s, T> {
    topic: &'s SystemTopic<T>,
}

impl<'s, T> From<&'s SystemTopic<T>> for SystemTopicReader<'s, T> {
    fn from(topic: &'s SystemTopic<T>) -> Self {
        Self { topic }
    }
}

pub struct SystemTopicReaderState<T: Resource> {
    topic_id: Entity,
    _t: PhantomData<T>,
}

impl<T: Resource> SystemTopic<T> {
    pub fn update_system(mut query: Query<&mut SystemTopic<T>>) {
        for mut t in query.iter_mut() {
            let t = &mut *t;
            std::mem::swap(&mut t.read_events, &mut t.write_events);
            t.write_events.clear();
        }
    }
}

#[async_trait]
impl<'s, T: Sync + Send> TopicReader<'s, T> for SystemTopicReader<'s, T> {
    async fn read(self) -> AnyResult<ReadGuard<'s, T>> {
        if !self.topic.read_events.is_empty() {
            trace!("{}::read()", std::any::type_name::<Self>());
        }
        // result reserved for future compatibility
        Ok(ReadGuard::new(GuardedEvents::Slice(
            &self.topic.read_events[0..self.topic.read_events.len()],
        )))
    }

    fn try_read(self) -> Option<ReadGuard<'s, T>> {
        if !self.topic.read_events.is_empty() {
            trace!("{}::try_read()", std::any::type_name::<Self>());
            return Some(ReadGuard::new(GuardedEvents::Slice(
                &self.topic.read_events[0..self.topic.read_events.len()],
            )));
        }
        None
    }
}

impl<'s, T: Resource> PipeParam for SystemTopicReader<'s, T> {
    type State = SystemTopicReaderState<T>;
}

#[async_trait]
impl<T: Resource> PipeParamState for SystemTopicReaderState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Reader;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.get_topic_id(Self::type_name())?;
        Ok(Self {
            topic_id: id,
            _t: Default::default(),
        })
    }
}

impl<'w: 's, 's, T: Resource> PipeSystemParamFetch<'w, 's> for SystemTopicReaderState<T> {
    type PipeParam = SystemTopicReader<'s, T>;
    type SystemParam = BevySystemTopicReader<'w, 's, T>;

    fn get_param(&'s mut self, system_param: &'s mut Self::SystemParam) -> Self::PipeParam {
        SystemTopicReader {
            topic: self.topic_id.get(&system_param.query),
        }
    }
}

impl<T: Resource> PipeObserver for SystemTopicReaderState<T> {
}

#[derive(SystemParam)]
pub struct BevySystemTopicWriter<'w, 's, T: Resource> {
    query: Query<'w, 's, &'static mut SystemTopic<T>>,
}

#[derive(SystemParam)]
pub struct GlobalSystemWriter<'w, 's, T: Resource> {
    query: Query<'w, 's, &'static mut SystemTopic<T>>,
    global_id: Res<'w, GlobalEntity>,
}

impl<'w, 's, T: Resource> GlobalSystemWriter<'w, 's, T> {
    pub fn write(&mut self, data: T) {
        trace!("{}::write", Self::type_name());
        self.global_id
            .get_mut(&mut self.query)
            .write_events
            .push(data);
    }
}

impl<'w, 's, T: Resource> Extend<T> for GlobalSystemWriter<'w, 's, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.global_id
            .get_mut(&mut self.query)
            .write_events
            .extend(iter);
    }
}

pub struct SystemTopicWriterState<T: Resource> {
    topic_id: Entity,
    _t: PhantomData<T>,
}

pub struct SystemTopicWriter<'s, T> {
    topic: Mut<'s, SystemTopic<T>>,
}

impl<'s, T: Resource> PipeParam for SystemTopicWriter<'s, T> {
    type State = SystemTopicWriterState<T>;
}

#[async_trait]
impl<T: Resource> PipeParamState for SystemTopicWriterState<T> {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::Reader;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.get_topic_id(Self::type_name())?;
        Ok(Self {
            topic_id: id,
            _t: Default::default(),
        })
    }
}

impl<'w: 's, 's, T: Resource> PipeSystemParamFetch<'w, 's> for SystemTopicWriterState<T> {
    type PipeParam = SystemTopicWriter<'s, T>;
    type SystemParam = BevySystemTopicWriter<'w, 's, T>;

    fn get_param(&'s mut self, system_param: &'s mut Self::SystemParam) -> Self::PipeParam {
        SystemTopicWriter {
            topic: self.topic_id.get_mut(&mut system_param.query),
        }
    }
}

impl<T: Resource> PipeObserver for SystemTopicWriterState<T> {
}
