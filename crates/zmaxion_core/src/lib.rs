pub use bevy_ecs;
pub use smallvec;

pub mod components;
// pub mod error;
pub mod macros;
pub mod messages;
pub mod models;
// pub mod pipe;
pub mod resources;

// pub mod topic;

pub mod prelude {
    pub use bevy_derive::{Deref, DerefMut};
    pub use bevy_ecs::{self, prelude::*, system::Resource};
    pub use bevy_utils::{HashMap, HashSet};
    pub use derive_new::new;

    // If schedule is rebuilt then these trackers could trigger systems thus making them
    // private
    //    #[allow(unused_imports)]
    //    use bevy::prelude::{Added, Changed, RemovedComponents};
    //    #[allow(unused_imports)]
    //    use bevy::prelude::{DefaultPlugins, Plugin, PluginGroup};
    //    pub use bevy::{
    //        ecs as bevy_ecs,
    //        prelude::{Plugin as BevyPlugin, PluginGroup as BevyPluginGroup, *},
    //        utils::{HashMap, HashSet},
    //    };
    pub use crate::{
        components::Name, read, read_all, read_all_loop, read_loop, resources::GlobalEntity,
    };
}
