pub use bevy;
pub use smallvec;

pub mod definitions;
pub mod error;
pub mod macros;
pub mod pipe;
pub mod pipeline;
pub mod state;
pub mod sync;
pub mod topic;

#[doc(hidden)]
pub mod prelude {
    // If schedule is rebuilt then these trackers could trigger systems thus making them
    // private
    #[allow(unused_imports)]
    use bevy::prelude::{Added, Changed, RemovedComponents};
    #[allow(unused_imports)]
    use bevy::prelude::{DefaultPlugins, Plugin, PluginGroup};
    pub use bevy::prelude::{Plugin as BevyPlugin, PluginGroup as BevyPluginGroup, *};
    pub use zmaxion_rt::prelude::*;
    pub use zmaxion_utils::prelude::*;

    pub use crate::{
        components::Name,
        error::{anyhow, bail, ensure, AnyResult, OptionIntoResultExt},
        read, read_all, read_all_loop, read_loop,
        resources::GlobalEntity,
        topic::components::{ResTopicReader, ResTopicWriter, TopicReader, TopicWriter},
    };
}

pub mod components {
    use crate::prelude::Component;

    #[derive(Component)]
    pub struct Name(pub String);
    #[derive(Component)]
    pub struct Generation(pub u64);
}

pub mod resources {
    use std::sync::{atomic::AtomicBool, Arc};

    use bevy::{prelude::Entity, utils::HashSet};

    pub struct GlobalEntity(pub Entity);
    //    pub struct GlobalReadEntity(pub Entity);
    //    pub struct GlobalWriteEntity(pub Entity);
    pub struct LoadedConnectors(pub HashSet<String>);
    pub struct LogErrorsSynchronously(pub Arc<AtomicBool>);
}
