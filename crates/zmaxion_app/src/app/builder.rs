use std::{mem::swap, sync::Arc};

pub use zmaxion_core::prelude::*;
use zmaxion_core::{
    bevy::{
        ecs::{
            schedule::IntoSystemDescriptor,
            system::{Resource, SystemParam, SystemParamFetch, SystemState},
        },
        reflect::{GetTypeRegistration, TypeRegistryArc},
    },
    definitions::StaticEstimations,
    error::handle_errors,
    pipe::{
        factory::{IntoPipeFactory, PipeFactoryContainer},
        resources::PipeDefs,
    },
    pipeline::messages::SpawnPipeline,
    resources::LoadedConnectors,
    sync::{PrioMutex, PrioMutexGuard},
    topic::{resources::TopicDefinitions, MemTopic, TopicDefinition, TopicSpawnerArgs},
};

use crate::app::Schedules;
pub use crate::prelude::*;

pub struct AppBuilder<'a> {
    pub schedules: &'a mut [Schedule; 3],
    pub world: PrioMutexGuard<'a, World>,
    pub world_arc: Arc<PrioMutex<World>>,
}

impl<'a> AppBuilder<'a> {
    pub fn new(app: &'a mut Zmaxion) -> Self {
        Self {
            schedules: &mut app.schedules,
            world: app.world.lock(0).unwrap(),
            world_arc: app.world.clone(),
        }
    }

    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: Plugin,
    {
        debug!("added plugin: {}", plugin.name());
        let plugin = Box::new(plugin);
        plugin.build(self);
        self
    }

    pub fn add_plugins<T: PluginGroup>(&mut self, mut group: T) -> &mut Self {
        let mut plugin_group_builder = PluginGroupBuilder::default();
        group.build(&mut plugin_group_builder);
        plugin_group_builder.finish(self);
        self
    }

    pub fn add_bevy_plugin<T: BevyPlugin>(&mut self, plugin: T) -> &mut Self {
        self.add_bevy_plugin_or_group(move |app| app.add_plugin(plugin))
    }

    pub fn add_bevy_plugins<T: BevyPluginGroup>(&mut self, group: T) -> &mut Self {
        self.add_bevy_plugin_or_group(move |app| app.add_plugins(group))
    }

    pub unsafe fn register_type<T: GetTypeRegistration>(&mut self) -> &mut Self {
        // TODO: skip registering if already registered
        let registry = self.world.get_resource_mut::<TypeRegistryArc>().unwrap();
        registry.write().register::<T>();
        self
    }

    /// Plugins that create custom topic must register the name of a topic kind
    pub fn add_connector(&mut self, kind: impl Into<String>) -> &mut Self {
        if !self
            .world
            .get_resource_mut::<LoadedConnectors>()
            .unwrap()
            .0
            .insert(kind.into())
        {
            panic!("Connector already registered");
        }
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_topic<T: Resource>(&mut self) -> &mut Self {
        self.register_topic::<T>();
        let id = self.world.get_resource::<GlobalEntity>().unwrap().0;
        let topic = MemTopic::<T>::new();
        self.world.entity_mut(id).insert(topic);
        self
    }

    //    pub fn register_state<T: Resource>(&mut self) -> &mut Self {
    //        self.register_topic::<Stateful<T>>()
    //    }

    pub fn register_topic<T: Resource>(&mut self) -> &mut Self {
        fn topic_spawner<T: Resource>(args: &mut TopicSpawnerArgs) {
            debug!("{:#?}", T::type_name());
            debug!("{:#?}", args.commands.id());
            args.commands.insert(MemTopic::<T>::new());
        }
        let mut topic_systems = self.world.get_resource_mut::<TopicDefinitions>().unwrap();
        let system = MemTopic::<T>::update_system;

        if topic_systems
            .0
            .insert(
                T::type_name().to_string(),
                TopicDefinition {
                    spawner: topic_spawner::<T>,
                },
            )
            .is_none()
        {
            self.schedules[Schedules::Post as usize].add_system_to_stage(CoreStage::Last, system);
        }
        self
    }

    pub fn init_resource<R: FromWorld + Send + Sync + 'static>(&mut self) -> &mut Self {
        let resource = R::from_world(&mut *self.world);
        self.world.insert_resource(resource);
        self
    }

    pub fn set_schedule(&'a mut self, schedule: Schedules) -> SystemAdder<'a, CoreStage> {
        SystemAdder {
            builder: self,
            current_stage: CoreStage::Update,
            current_schedule: schedule as usize,
        }
    }

    pub fn set_stage<S: StageLabel>(&'a mut self, stage: S) -> SystemAdder<'a, S> {
        SystemAdder {
            builder: self,
            current_stage: stage,
            current_schedule: Schedules::Post as usize,
        }
    }

    //    pub fn add_raw_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
    //        self.inner.add_plugin(plugin);
    //        self
    //    }

    pub fn add_system<Params>(&mut self, desc: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.schedules[Schedules::Post as usize]
            .stage(CoreStage::Update, |stage: &mut SystemStage| {
                stage.add_system(desc)
            });
        self
    }

    pub fn add_try_system<S, Out: 'static, Params>(&mut self, desc: S) -> &mut Self
    where
        S: IntoSystem<(), AnyResult<Out>, Params>,
    {
        self.add_system(desc.chain(handle_errors::<Out>));
        self
    }

    pub fn add_startup_system<Params>(
        &mut self,
        desc: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.schedules[Schedules::Post as usize]
            .stage(CoreStage::Startup, |schedule: &mut Schedule| {
                schedule.add_system_to_stage(StartupStage::Startup, desc)
            });
        self
    }

    pub fn add_startup_try_system<S, Out: 'static, Params>(&mut self, desc: S) -> &mut Self
    where
        S: IntoSystem<(), AnyResult<Out>, Params>,
    {
        self.add_startup_system(desc.chain(handle_errors::<Out>));
        self
    }

    pub fn register_pipe<Out, Params, Marker, S: IntoPipeFactory<Out, Params, Marker> + 'static>(
        &mut self,
        s: S,
        estimations: Option<StaticEstimations>,
    ) -> &mut Self {
        let mut defs = self.world.get_resource_mut::<PipeDefs>().unwrap();
        let name = S::type_name();
        if defs.0.contains_key(name) {
            panic!("Pipe `{}` already registered", name);
        }
        defs.0.insert(
            name,
            PipeFactoryContainer {
                factory: s.into_pipe_factory(),
                kind: S::KIND,
                estimations,
            },
        );
        self
    }

    fn add_bevy_plugin_or_group<F: FnOnce(&mut App) -> &mut App>(&mut self, mut f: F) -> &mut Self {
        let mut app = App::empty();
        swap(&mut app.world, &mut *self.world);
        swap(
            &mut app.schedule,
            &mut self.schedules[Schedules::Post as usize],
        );
        debug!(
            "add {:#?}",
            app.schedule.iter_stages().map(|x| x.0).collect::<Vec<_>>()
        );
        f(&mut app);
        swap(&mut app.world, &mut *self.world);
        swap(
            &mut app.schedule,
            &mut self.schedules[Schedules::Post as usize],
        );
        self
    }
}

pub struct SystemAdder<'a, StageLabel> {
    builder: &'a mut AppBuilder<'a>,
    current_stage: StageLabel,
    current_schedule: usize,
}

impl<'a, Label: StageLabel + Clone> SystemAdder<'a, Label> {
    pub fn add_system<Params>(mut self, desc: impl IntoSystemDescriptor<Params>) -> Self {
        self.builder.schedules[self.current_schedule]
            .stage(self.current_stage.clone(), |stage: &mut SystemStage| {
                stage.add_system(desc)
            });
        self
    }

    pub fn add_startup_system<Params>(mut self, desc: impl IntoSystemDescriptor<Params>) -> Self {
        let stage = self.current_stage.clone();
        self.builder.schedules[self.current_schedule]
            .stage(CoreStage::Startup, |schedule: &mut Schedule| {
                schedule.add_system_to_stage(stage, desc)
            });
        self
    }

    pub fn set_schedule(mut self, schedule: Schedules) -> Self {
        self.current_schedule = schedule as usize;
        self
    }

    pub fn set_stage<S: StageLabel>(self, stage: S) -> SystemAdder<'a, S> {
        SystemAdder {
            builder: self.builder,
            current_stage: stage,
            current_schedule: self.current_schedule,
        }
    }
}
impl<'a, T> SystemAdder<'a, T> {
    pub fn finish(self) -> &'a mut AppBuilder<'a> {
        self.builder
    }
}
