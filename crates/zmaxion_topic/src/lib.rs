use std::{
    any::{Any, TypeId},
    sync::{atomic::AtomicU8, Arc},
};

use zmaxion_core::{
    bevy_ecs::system::{SystemMeta, SystemParamFetch, SystemParamState},
    prelude::*,
};

use crate::dyn_topic::{ReadGuard, TopicReader, TopicWriter};

#[doc(hidden)]
mod __export__ {
    pub use async_trait::async_trait;
    pub use zmaxion_core::prelude::Entity;
    pub use zmaxion_param::{
        ParamBuilder, PipeParamImpl, PipeParamStateImpl, TopicParam, TopicParamKind,
    };
    pub use zmaxion_utils::prelude::AnyResult;
}
pub mod system_df_topic;

// mod api;
mod async_topic;
// pub mod topic_v2;
// pub mod data_frame_topic;
// mod df_topic;
pub mod dyn_topic;
// mod topic_impl;
// pub use topic_impl::*;
mod models;
mod system_topic;

pub use system_topic::SystemTopic;
use zmaxion_core::models::TopicSpawnerArgs;
use zmaxion_param::{AsyncReader, AsyncReaderState, AsyncTopic};
use zmaxion_rt::AsyncMutex;
use zmaxion_utils::prelude::RwLock;

use crate::system_topic::{SystemTopicReader, SystemTopicReaderState};

pub mod prelude {
    pub use zmaxion_param::{AsyncReader, AsyncWriter};

    pub use crate::{
        async_topic::{GlobalAsyncReader, GlobalAsyncWriter},
        dyn_topic::{DynReader, DynWriter, TopicReader, TopicWriter},
        system_df_topic::{
            GenericGlobalBevyReader, GenericGlobalBevyWriter, GlobalBevyReader, GlobalBevyWriter,
        },
        system_topic::{
            BevySystemTopicWriter, GlobalSystemReader, GlobalSystemWriter, SystemTopicReader,
            SystemTopicWriter,
        },
    };
}
pub mod components {
    use zmaxion_core::prelude::*;

    #[derive(Component, Deref, DerefMut)]
    pub struct TopicRwBuilderId(usize);
}
pub mod resources {
    use zmaxion_core::prelude::*;
    use zmaxion_utils::prelude::AnyResult;

    use crate::dyn_topic::{DynReaderState, DynWriter};

    #[derive(Deref, DerefMut)]
    pub struct TopicReaderBuilders<T>(Vec<fn(&mut World, Entity) -> AnyResult<DynReaderState<T>>>);
    #[derive(Deref, DerefMut)]
    pub struct TopicWriterBuilders<T>(Vec<fn(&mut World, Entity) -> AnyResult<DynWriter<T>>>);
}
pub struct TopicFeatures {
    /// Can be used for Bevy's system
    pub system_execution: bool,
    /// Can be used in async runtime
    pub async_execution: bool,
    /// Wether the cursor is saved or it is reset after each epoch
    pub cursors_cached: bool,
    pub semantics: Semantics,
}
pub trait HasTopicFeatures {
    const FEATURES: TopicFeatures;
    fn add_system(stage: &mut SystemStage);
}
/// Semantics of a topic if a pipe would be executed again. Not having multiple read calls in a
/// pipe.
pub enum Semantics {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}
pub trait TopicRwState {
    type Topic;
    fn from_topic(topic: &Self::Topic) -> Self;
}

pub struct TopicRegistration {}

pub enum TopicReaderStateEnum<T: Resource> {
    System(SystemTopicReaderState<T>),
    Async(AsyncReaderState<T>),
}

pub struct ReaderState<T: Resource>(Arc<AsyncMutex<TopicReaderStateEnum<T>>>);
