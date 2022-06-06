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

impl<T> HasTopicFeatures for SystemTopic<T> {
    const FEATURES: TopicFeatures = TopicFeatures {
        system_execution: true,
        async_execution: false,
        cursors_cached: false,
        semantics: Semantics::AtMostOnce,
    };
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

impl<'w, 's, T: Resource> GlobalSystemReader<'w, 's, T> {
    pub fn read(&self) -> &[T] {
        &self.query.get(**self.global_id).unwrap().read_events
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
    pub fn system(mut query: Query<&mut SystemTopic<T>>) {
        for mut t in query.iter_mut() {
            let t = &mut *t;
            std::mem::swap(&mut t.read_events, &mut t.write_events);
            t.write_events.clear();
        }
    }
}

#[async_trait]
impl<'s, T: Sync + Send> TopicReader<'s, T> for SystemTopicReader<'s, T> {
    async fn read(self) -> ReadGuard<'s, T> {
        self.try_read()
    }

    fn try_read(self) -> ReadGuard<'s, T> {
        let range = 0..self.topic.read_events.len();
        if !self.topic.read_events.is_empty() {
            trace!("{}::try_read()", std::any::type_name::<Self>());
        }
        ReadGuard::new(GuardedEvents::Slice(&self.topic.read_events), range)
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
        let world = builder.world();
        let topic = world.get::<SystemTopic<T>>(id).unwrap();
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
            topic: system_param.query.get(self.topic_id).unwrap(),
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
        self.query
            .get_mut(**self.global_id)
            .unwrap()
            .write_events
            .push(data);
    }
}

impl<'w, 's, T: Resource> Extend<T> for GlobalSystemWriter<'w, 's, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.query
            .get_mut(**self.global_id)
            .unwrap()
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
        let world = builder.world();
        let topic = world.get::<SystemTopic<T>>(id).unwrap();
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
            topic: system_param.query.get_mut(self.topic_id).unwrap(),
        }
    }
}

impl<T: Resource> PipeObserver for SystemTopicWriterState<T> {
}
