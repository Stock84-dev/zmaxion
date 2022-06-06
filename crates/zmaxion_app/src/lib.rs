mod app;
mod plugin;
mod topic_plugin;

#[doc(hidden)]
pub mod prelude {
    pub use bevy_app::prelude::{CoreStage, Plugin as BevyPlugin, PluginGroup as BevyPluginGroup};

    pub use crate::{
        app::{AppBuilder, Zmaxion},
        plugin::{Plugin, PluginGroup, PluginGroupBuilder},
    };
}
