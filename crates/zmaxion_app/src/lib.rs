mod app;
mod plugin;

#[doc(hidden)]
pub mod prelude {
    pub use crate::app::{AppBuilder, Zmaxion};
    pub use crate::plugin::{Plugin, PluginGroup, PluginGroupBuilder};
}

pub mod resources {
    use std::sync::Arc;
    use zmaxion_core::bevy::utils::HashSet;
    use zmaxion_core::prelude::World;
    use zmaxion_core::sync::PrioMutex;

    pub struct WorldArc(pub Arc<PrioMutex<World>>);
    pub struct LoadedConnectors(pub HashSet<String>);
    pub struct Reschedule;
    pub struct Exit;
}
