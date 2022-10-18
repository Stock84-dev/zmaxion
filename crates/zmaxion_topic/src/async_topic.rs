use std::{
    marker::PhantomData,
    ops::{DerefMut, Range},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bevy_ecs::{
    archetype::Archetype,
    system::{Resource, SystemMeta, SystemParamFetch, SystemParamState},
};
use cache_padded::CachePadded;
use zmaxion_core::{components::Arcc, prelude::*};
use zmaxion_param::{
    prelude::*, AsyncReader, AsyncReaderState, AsyncTopic, AsyncTopicInner, AsyncWriter,
    AsyncWriterState, ParamBuilder, PipeObserver, PipeParam, PipeParamFetch, PipeParamImpl,
    PipeParamState, PipeParamStateImpl, TopicParamKind,
};
use zmaxion_rt::prelude::*;
use zmaxion_utils::prelude::*;

use crate::{
    bevy_ecs::system::SystemParam,
    dyn_topic::{GuardedEvents, ReadGuard, ReaderState, TopicReader, TopicWriter, WriterState},
    HasTopicFeatures, Semantics, TopicFeatures,
};

impl<T: Resource> HasTopicFeatures for AsyncTopic<T> {
    const FEATURES: TopicFeatures = TopicFeatures {
        system_execution: true,
        async_execution: true,
        cursors_cached: true,
        semantics: Semantics::AtMostOnce,
    };

    fn add_system(stage: &mut SystemStage) {
        stage.add_system(Self::update_system);
    }
}

#[async_trait]
impl<T: Resource> ReaderState<T> for AsyncReaderState<T> {
    async unsafe fn read<'a>(&'a mut self) -> AnyResult<ReadGuard<'a, T>> {
        let results = self.read_raw().await;
        // for future compatibility
        Ok(ReadGuard::new(GuardedEvents::Guard(results.0, results.1)))
    }

    unsafe fn try_read<'a>(&'a mut self) -> Option<ReadGuard<'a, T>> {
        let results = self.try_read_raw();
        if results.1 == Range::default() {
            return None;
        }
        Some(ReadGuard::new(GuardedEvents::Guard(results.0, results.1)))
    }
}

impl<'s, T: Resource> WriterState<T> for AsyncWriterState<T> {
    unsafe fn extend_from_slice(&mut self, data: &[T])
    where
        T: Clone,
    {
        Self::extend_from_slice(self, data);
    }

    unsafe fn extend_one(&mut self, value: T) {
        Self::extend_one(self, value);
    }

    unsafe fn extend<I>(&mut self, iter: I)
    where
        Self: Sized,
        I: IntoIterator<Item = T>,
    {
        Self::extend(self, iter);
    }

    unsafe fn extend_dyn(&mut self, iter: &mut dyn Iterator<Item = T>) {
        Self::extend(self, iter);
    }
}

//#[derive(SystemParam)]
// pub struct GlobalAsyncReader<'w, 's, T: Resource> {
//    query: Query<'w, 's, &'static mut AsyncReaderState<T>>,
//    global_id: Res<'w, GlobalEntity>,
//}
// impl<'w, 's, T: Resource> GlobalAsyncReader<'w, 's, T> {
//    pub async fn read(&'s mut self) -> ReadGuard<'s, T> {
//        let state = self.global_id.get_mut(&mut self.query).into_inner();
//        unsafe { state.read().await.unwrap() }
//    }
//
//    pub fn try_read(&'s mut self) -> Option<ReadGuard<'s, T>> {
//        let state = self.global_id.get_mut(&mut self.query).into_inner();
//        unsafe { state.try_read() }
//    }
//}

pub struct GlobalAsyncWriter<'s, T: Resource> {
    writer: &'s AsyncWriterState<T>,
}

impl<'s, T: Resource> SystemParam for GlobalAsyncWriter<'s, T> {
    type Fetch = GlobalAsyncWriterState<T>;
}
#[doc(hidden)]
pub struct GlobalAsyncWriterState<T: Resource> {
    state: AsyncWriterState<T>,
}
unsafe impl<T: Resource> SystemParamState for GlobalAsyncWriterState<T> {
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let id = world.resource::<GlobalEntity>();
        let topic = world.try_ref::<AsyncTopic<T>>(id.0).unwrap();

        Self {
            state: AsyncWriterState::from(topic.clone()),
        }
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
    }

    fn apply(&mut self, world: &mut World) {
    }
}
impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for GlobalAsyncWriterState<T> {
    type Item = GlobalAsyncWriter<'s, T>;

    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        GlobalAsyncWriter {
            writer: &mut state.state,
        }
    }
}

impl<'s, T: Resource> GlobalAsyncWriter<'s, T> {
    pub fn extend_from_slice(&self, data: &[T])
    where
        T: Clone,
    {
        unsafe { self.writer.extend_from_slice(data) }
    }

    pub fn extend_one(&self, value: T) {
        unsafe { self.writer.extend_one(value) }
    }

    pub fn extend<I: IntoIterator<Item = T>>(&self, iter: I) {
        unsafe { self.writer.extend(iter) }
    }
}

pub struct GlobalAsyncReader<'s, T: Resource> {
    reader: &'s mut AsyncReaderState<T>,
}

impl<'s, T: Resource> SystemParam for GlobalAsyncReader<'s, T> {
    type Fetch = GlobalAsyncReaderState<T>;
}
#[doc(hidden)]
pub struct GlobalAsyncReaderState<T: Resource> {
    state: AsyncReaderState<T>,
}
unsafe impl<T: Resource> SystemParamState for GlobalAsyncReaderState<T> {
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
        let id = world.resource::<GlobalEntity>();
        let topic = world.try_ref::<AsyncTopic<T>>(id.0).unwrap();

        Self {
            state: AsyncReaderState::from(topic.clone()),
        }
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
    }

    fn apply(&mut self, world: &mut World) {
    }
}
impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for GlobalAsyncReaderState<T> {
    type Item = GlobalAsyncReader<'s, T>;

    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        GlobalAsyncReader {
            reader: &mut state.state,
        }
    }
}

impl<'s, T: Resource> GlobalAsyncReader<'s, T> {
    pub async fn read<'a>(&'a mut self) -> ReadGuard<'a, T> {
        unsafe { self.reader.read().await }.unwrap()
    }

    pub fn try_read<'a>(&'a mut self) -> Option<ReadGuard<'a, T>> {
        unsafe { self.reader.try_read() }
    }
}
