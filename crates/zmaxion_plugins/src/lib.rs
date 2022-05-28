mod pipe;
mod pipeline;
mod state;
mod topic;

pub mod prelude {
    pub use crate::DefaultPlugins;
}

use bevy::reflect::TypeRegistryArc;
use zmaxion_app::{prelude::*, resources::WorldArc};
use zmaxion_core::{
    components::Name,
    error::ErrorEvent,
    prelude::*,
    topic::{MemTopic, TopicReaderState},
};
use zmaxion_core::resources::LogErrorsSynchronously;

use crate::state::StatePlugin;
pub use crate::{pipe::PipePlugin, pipeline::PipelinePlugin, topic::TopicPlugin};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        let global = builder.world.spawn().insert(Name("Global".into())).id();
        //        let global_read = builder.world.spawn().insert(Name("GlobalRead".into())).id();
        //        let global_write = builder
        //            .world
        //            .spawn()
        //            .insert(Name("GlobalWrite".into()))
        //            .id();
        let world = builder.world_arc.clone();
        builder
            .insert_resource(GlobalEntity(global))
            .insert_resource(TypeRegistryArc::default())
            .insert_resource(WorldArc(world))
            .add_bevy_plugin(zmaxion_core::bevy::core::CorePlugin)
    }
}

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        let id = builder.world.get_resource::<GlobalEntity>().unwrap().0;
        builder.add_topic::<ErrorEvent>().insert_resource(LogErrorsSynchronously(Default::default()));
        let topic = builder
            .world
            .entity(id)
            .get::<MemTopic<ErrorEvent>>()
            .unwrap()
            .clone();
        builder
            .world
            .entity_mut(id)
            .insert(TopicReaderState::from(topic));
        builder
    }
}

pub struct LogErrorsPlugin;

impl Plugin for LogErrorsPlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder.add_system(log_errors)
    }
}

fn log_errors(errors: Query<(&TopicReaderState<ErrorEvent>, Option<&Name>, Entity)>) {
    for (topic, name, id) in errors.iter() {
        let mut guard = read_loop!(topic);
        error!(
            "{:?} for `{}`({:?}) pipeline",
            guard.try_read().unwrap().error,
            name.unwrap_or(&Name("Unknown".into())).0,
            id
        );
    }
}

pub struct MinimalPlugins;

pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build<'a>(&'a mut self, group: &'a mut PluginGroupBuilder) -> &'a mut PluginGroupBuilder {
        group
            .add(CorePlugin)
            .add(AsyncRuntimePlugin::default())
            .add(TopicPlugin)
            .add(PipePlugin)
            .add(PipelinePlugin)
            .add(StatePlugin)
            .add(ErrorPlugin)
    }
}

#[derive(Default)]
pub struct AsyncRuntimePlugin(pub Runtime);

impl Plugin for AsyncRuntimePlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        zmaxion_rt::set_runtime(&self.0);
        builder.insert_resource(self.0)
    }
}
