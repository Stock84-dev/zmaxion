use std::{
    any::{Any, TypeId},
    fmt::{Formatter, Write},
    mem::swap,
    sync::Arc,
};

use zmaxion_core::{
    bevy::{
        diagnostic::{DiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
        ecs::{
            schedule::IntoSystemDescriptor,
            system::{Resource, SystemMeta, SystemState},
        },
        reflect::{GetTypeRegistration, TypeRegistryArc},
        utils::HashSet,
    },
    components::Name,
    definitions::{PipeKind, StaticEstimations},
    error::{handle_errors, ErrorEvent},
    pipe::{
        components::PipeComponent,
        factory::{IntoPipeFactory, PipeFactoryContainer},
        messages::AddSystem,
        resources::PipeDefs,
    },
    prelude::*,
    sync::PrioMutex,
    topic::{
        resources::TopicDefinitions, MemTopic, ResTopicReaderState, TopicDefinition,
        TopicReaderState, TopicSpawnerArgs,
    },
};

use crate::{
    prelude::*,
    resources::{Exit, LoadedConnectors, Reschedule},
};

mod builder;

pub use builder::AppBuilder;
use zmaxion_core::pipeline::messages::SpawnPipeline;

pub enum Schedules {
    Pre = 0,
    Main = 1,
    Post = 2,
}

pub struct Zmaxion {
    pub schedules: [Schedule; 3],
    pub world: Arc<PrioMutex<World>>,
}

impl Zmaxion {
    // TODO: use https://crates.io/crates/nested
    // TODO: use https://lib.rs/crates/slice-dst
    // TODO: use flatbuffers
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        Self {
            schedules: [Default::default(), Default::default(), Default::default()],
            world: Arc::new(PrioMutex::new(World::new())),
        }
    }

    pub fn builder<'a>(&'a mut self) -> AppBuilder<'a> {
        AppBuilder::new(self)
    }

    /// Runs startup stages, does nothing if already called
    pub fn startup(&mut self) {
        let mut world = self.world.lock(0).unwrap();
        for schedule in &mut self.schedules {
            schedule.stage(CoreStage::Startup, |schedule: &mut Schedule| {
                schedule.run(&mut *world);
                schedule
            });
        }
    }

    pub fn spawn_pipeline(&self, config: SpawnPipeline) {
        let mut world = self.world.lock(0).unwrap();
        let mut system_state: SystemState<ResTopicWriter<SpawnPipeline>> =
            SystemState::new(&mut *world);
        let writer = system_state.get_mut(&mut *world);
        writer.write(config);
    }

    pub fn run(&mut self) {
        let topic = {
            let mut world = self.world.lock(0).unwrap();
            ResTopicReaderState::<AddSystem>::from(&*world)
        };
        self.startup();
        let mut epoch = 0;
        loop {
            trace!("Epoch: {}", epoch);
            let mut world = self.world.lock(usize::MAX).unwrap();
            for schedule in &mut self.schedules {
                schedule.run(&mut world);
            }

            if world.remove_resource::<Reschedule>().is_some() {
                self.schedules[Schedules::Main as usize] = Schedule::default();
                let mut query = world.query::<(&PipeComponent, &Name)>();
                for (component, name) in query.iter(&world) {
                    if let Some(system) = component.factory.system() {
                        let defs = world.get_resource::<PipeDefs>().unwrap();
                        match &defs.0.get(name.0.as_str()).unwrap().kind {
                            PipeKind::Bevy => {
                                self.schedules[Schedules::Main as usize]
                                    .add_system_to_stage(CoreStage::Update, system);
                            }
                            Async => {}
                        }
                    }
                }
            } else {
                let mut system_state: SystemState<(Res<PipeDefs>, Query<(&PipeComponent, &Name)>)> =
                    SystemState::new(&mut *world);
                let (defs, query) = system_state.get_mut(&mut *world);
                if let Some(mut commands) = topic.try_read() {
                    for system in commands.read_all() {
                        let (component, name) = query.get(system.entity).unwrap();
                        if let Some(system) = component.factory.system() {
                            match &defs.0.get(name.0.as_str()).unwrap().kind {
                                PipeKind::Bevy => {
                                    self.schedules[Schedules::Main as usize]
                                        .add_system_to_stage(CoreStage::Update, system);
                                }
                                Async => {}
                            }
                        }
                    }
                }; // first drop temp variables from if statement then outer block
            }
            if world.remove_resource::<Exit>().is_some() {
                break;
            }
            epoch += 1;
        }
    }
}

impl Default for Zmaxion {
    fn default() -> Self {
        let main_app = App::default();
        let mut app0 = App::empty();
        app0.add_default_stages();
        let mut app1 = App::empty();
        app1.add_default_stages();
        let mut zmaxion = Zmaxion {
            schedules: [app0.schedule, app1.schedule, main_app.schedule],
            world: Arc::new(PrioMutex::new(main_app.world)),
        };
        zmaxion
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::{log::*, prelude::*, utils::tracing};
    use mouse::prelude::set_tokio_handle;
    use tracing_subscriber::fmt::format::FmtSpan;
    use zmaxion_core::pipeline::messages::DespawnPipeline;

    use crate::{
        hello::HelloPlugin,
        pipeline::{DespawnPipeline, SpawnPipeline, SpawnPipelineInner},
        state::StatePlugin,
        GlobalEntity, MemTopic, Zion,
    };

    #[test]
    fn test() {
        tracing_subscriber::fmt()
            // enable everything
            .with_max_level(tracing::Level::TRACE)
            // display source code file paths
            .with_file(true)
            // display source code line numbers
            .with_line_number(true)
            // disable targets
            .with_target(false)
            .with_span_events(FmtSpan::ACTIVE)
            // sets this to be the default, global collector for this application.
            .init();
        //    use tracing_subscriber::layer::SubscriberExt;
        //    let layer = tracing_subscriber::fmt::layer()
        //        .with_span_events(FmtSpan::CLOSE)
        //        .finish();
        //    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer))
        //        .expect("set up the subscriber");
        //    use tracing::trace;
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(set_tokio_handle());
        let mut zion = Zion::new();
        zion.world.lock(0).unwrap().insert_resource(LogSettings {
            filter: "wgpu=trace".to_string(),
            level: Level::TRACE,
        });
        //    zion.add_legacy_plugin(LogPlugin);
        zion.add_plugin(HelloPlugin);
        let id = zion
            .world
            .lock(0)
            .unwrap()
            .get_resource::<GlobalEntity>()
            .unwrap()
            .0;
        let topic = zion
            .world
            .lock(0)
            .unwrap()
            .entity(id)
            .get::<MemTopic<DespawnPipeline>>()
            .unwrap();
        zion.run();
    }

    #[test]
    fn h() {
        #[derive(Component)]
        struct P;
        #[derive(Component)]
        struct C;
        let mut app = bevy::app::App::empty();
        app.add_default_stages()
            .add_startup_system(spawn)
            .add_system(p)
            .add_system(c);
        fn spawn(mut commands: Commands) {
            let parent = commands.spawn().id();
            commands.spawn().insert(Parent(parent));
        }
        fn p(query: Query<&Parent>) {
            println!("parents: {:#?}", query.iter().collect::<Vec<_>>());
        }

        fn c(query: Query<&Children>) {
            println!("children: {:#?}", query.iter().collect::<Vec<_>>());
        }
        app.run();
    }
}

// pipe could have a scale function which is called before spawning a pipeline. We could use that
// to determine how many pipe instances to run and set correct arguments.

// To spawn a workflow we send id to a topic.
// Then it gets picked up and moved to a priority topics, low, medium, high
// When reading workflows to spawn we read from high priority first
//
// Constraint: We cannot spawn a pipeline accross multiple nodes.
//
// TODO: implement state
// TODO: implement reader ack
// TODO: implement consumer groups to MemTopic<T>
// TODO: autoscale inside one node
// spawn the whole pipeline even the shared ones
// we can track execution time before/after
// if pipe can be executed concurrently and writers support out of order execution then try scale
// add one pipe and let it execute, compare execution speed
// if speed1 >= speed2 * 2 then keep it
// if we use wasi containers then we could keep track of memory consumption
// TODO: distributed autoscale
// we spawn a whole pipeline on one node, each pipe can be spawned on multiple nodes if it can be
// executed in parallel, if it crosses to another node then we upgrade topic
// if topic is in memory we don't store it
// if resource usage is too high, we can evict pipelines/pipe
// TODO: execution where data is located
// having higher replication factor makes things faster because we don't need to move data as much
// TODO: wasi containers

// visualization
// s3 -> load -> topic(mem)
// topic -> decompress -> topic # this one is stateless and should be merged up for best performance
// topic -> filter -> topic # also stateless
// topic -> draw_when_batched

// range_generator -> t -> backtest -> t -> compress -> t -> save_to_s3
//                                       -> filter   -> t -> draw_when_batched -> t -> save_to_s3
// if reader is statefull, attach generation to writer message
// pass all metadata through topics
// once a message has been procesed at the end of all pipe groups ack with stateful reader
// this requires that a pipe can only send one message per pipe invocation
