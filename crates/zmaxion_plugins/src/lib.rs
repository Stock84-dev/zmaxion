mod error;
mod pipe;
mod pipeline;
// mod state;
mod topic;

pub mod prelude {
    pub use crate::DefaultPlugins;
}

use tracing_subscriber::util::SubscriberInitExt;
use zmaxion_app::{prelude::*, BevyPluginWrapper};
use zmaxion_core::{
    components::Name,
    prelude::*,
    resources::{LogErrorsSynchronously, WorldArc},
};
use zmaxion_param::{
    prelude::{ErrorEvent, ErrorsState},
    AsyncReader, AsyncReaderState, AsyncTopic,
};
use zmaxion_rt::Runtime;
use zmaxion_topic::{dyn_topic::ReadGuard, prelude::*};
use zmaxion_utils::prelude::*;

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
        debug!("global entity: {:?}", global);
        let world = builder.world_arc.clone();
        builder
            .insert_resource(GlobalEntity(global))
            .insert_resource(WorldArc(world))
            .add_bevy_plugin(bevy_core::CorePlugin)
    }
}

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        let id = builder.world.get_resource::<GlobalEntity>().unwrap().0;
        builder
            .register_topic::<ErrorEvent>()
            .insert_resource(LogErrorsSynchronously(Default::default()));
        let topic = AsyncTopic::<ErrorEvent>::default();
        builder
            .world
            .entity_mut(id)
            .insert(topic.clone())
            .insert(AsyncReaderState::<ErrorEvent>::from(topic));
        builder
    }
}

pub struct LogErrorsPlugin;

impl Plugin for LogErrorsPlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder.add_system(log_errors)
    }
}

fn log_errors(mut errors: Query<(&mut AsyncReaderState<ErrorEvent>, Option<&Name>, Entity)>) {
    for (mut state, name, id) in errors.iter_mut() {
        let reader = AsyncReader::new(&mut *state);
        for e in read_all!(reader) {
            error!(
                "{:?} for `{}`({:?}) pipeline",
                e.error,
                name.unwrap_or(&Name("Unknown".into())).0,
                id
            );
        }
    }
}

pub struct MinimalPlugins;

pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build<'a>(&'a mut self, group: &'a mut PluginGroupBuilder) -> &'a mut PluginGroupBuilder {
        use tracing_subscriber::{filter::LevelFilter, fmt, fmt::format::FmtSpan};

        fmt()
            .with_max_level(LevelFilter::TRACE)
            .with_span_events(FmtSpan::FULL)
            //            .finish()
            //            .set_default()
            //            .event_format(fmt::format().compact())
            .init();
        group
            //            .add(BevyPluginWrapper(bevy_log::LogPlugin))
            .add(CorePlugin)
            .add(AsyncRuntimePlugin::default())
            .add(TopicPlugin::default())
            .add(PipePlugin)
            .add(PipelinePlugin)
            //            .add(StatePlugin)
            .add(ErrorPlugin)
    }
}

#[derive(Default)]
pub struct AsyncRuntimePlugin(pub Runtime);

impl Plugin for AsyncRuntimePlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        zmaxion_rt::set_runtime(&self.0);
        builder
    }
}

// bevy tracing starts at info level with release builds because it enables a compile time feature
// of a tracing crate
// change release profile without debug assertions once the fixed it
// https://github.com/bevyengine/bevy/issues/4069
#[cfg(test)]
todo_or_die::crates_io!("bevy", ">=0.8");
