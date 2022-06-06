use std::{
    ops::Range,
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
    prelude::*, AsyncTopic, AsyncTopicReader, AsyncTopicReaderState, AsyncTopicWriter,
    AsyncTopicWriterState, ParamBuilder, PipeObserver, PipeParam, PipeParamFetch, PipeParamImpl,
    PipeParamState, PipeParamStateImpl, TopicParamKind,
};
use zmaxion_rt::prelude::*;
use zmaxion_utils::prelude::*;

use crate::{
    dyn_topic::{GuardedEvents, ReadGuard, ReaderState, TopicReader, TopicWriter, WriterState},
    HasTopicFeatures, Semantics, TopicFeatures,
};

impl<T> HasTopicFeatures for AsyncTopic<T> {
    const FEATURES: TopicFeatures = TopicFeatures {
        system_execution: true,
        async_execution: true,
        cursors_cached: true,
        semantics: Semantics::AtMostOnce,
    };
}

#[async_trait]
impl<T: Resource> ReaderState<T> for AsyncTopicReaderState<T> {
    async unsafe fn read<'a>(&'a mut self) -> ReadGuard<'a, T> {
        let results = self.read_raw().await;
        ReadGuard::new(GuardedEvents::Guard(results.0), results.1)
    }

    unsafe fn try_read<'a>(&'a mut self) -> ReadGuard<'a, T> {
        let results = self.try_read_raw();
        ReadGuard::new(GuardedEvents::Guard(results.0), results.1)
    }
}

impl<'s, T: Resource> WriterState<T> for AsyncTopicWriterState<T> {
    unsafe fn write_slice(&mut self, data: &[T])
    where
        T: Clone,
    {
        self.write_slice_raw(data)
    }

    unsafe fn write(&mut self, value: T) {
        self.write_raw(value)
    }
}
