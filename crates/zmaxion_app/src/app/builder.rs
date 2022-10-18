use std::{mem::swap, sync::Arc};

use bevy_app::{App, StartupStage};
use bevy_ecs::{schedule::IntoSystemDescriptor, system::SystemState};
use bevy_reflect::{GetTypeRegistration, TypeRegistryArc};
use ergnomics::prelude::*;
pub use zmaxion_core::prelude::*;
use zmaxion_core::{
    models::{
        DynPipeFeatures, PipeDeclaration, PipeFeatures, StaticEstimations, TopicFactory,
        TopicHandler, TopicSpawnerArgs,
    },
    resources::{LoadedConnectors, TopicFactories, TopicHandlers},
};
use zmaxion_param::{prelude::handle_errors, AsyncTopic};
use zmaxion_pipe::{resources::PipeDefinitions, IntoPipeFactory, PipeDefinition};
use zmaxion_rt::{
    pipe_runtimes::RuntimeMarker, AsyncMutex, AsyncMutexGuard, BlockFutureExt, SpawnFutureExt,
};
use zmaxion_topic::{HasTopicFeatures, SystemTopic};
use zmaxion_utils::prelude::*;

use crate::app::Schedules;
pub use crate::prelude::*;

pub struct AppBuilder<'a> {
    pub schedules: &'a mut [Schedule; 3],
    pub world: AsyncMutexGuard<'a, World>,
    pub world_arc: Arc<AsyncMutex<World>>,
}

impl<'a> AppBuilder<'a> {
    pub fn new(app: &'a mut Zmaxion) -> Self {
        Self {
            schedules: &mut app.schedules,
            world: app.world.lock().block(),
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
        let mut connectors = self.world.get_resource_mut::<LoadedConnectors>().unwrap();
        let kind = kind.into();

        if connectors.0.contains(&kind) {
            panic!("Connector {} already registered", kind);
        }
        connectors.0.insert(kind);
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_system_topic<T: Resource>(&mut self) -> &mut Self {
        self.register_specific_topic::<SystemTopic<T>>();
        let id = self.world.get_resource::<GlobalEntity>().unwrap().0;
        let topic = SystemTopic::<T>::default();
        self.world.entity_mut(id).insert(topic);
        self
    }

    //    pub fn register_state<T: Resource>(&mut self) -> &mut Self {
    //        self.register_topic::<Stateful<T>>()
    //    }

    pub fn add_topic_handler(&mut self, handler: TopicHandler) {
        self.world.resource_mut::<TopicHandlers>().0.insert(handler);
    }

    pub fn register_specific_topic<T: Component + Default + HasTopicFeatures>(
        &mut self,
    ) -> &mut Self {
        fn topic_factory<T: Component + Default>(args: &mut TopicSpawnerArgs) {
            args.commands.insert(T::default());
        }
        let mut topic_factories = self.world.resource_mut::<TopicFactories>();

        if topic_factories
            .0
            .insert(
                T::type_name().to_string(),
                TopicFactory {
                    factory: topic_factory::<T>,
                },
            )
            .is_none()
        {
            let stage = stage_mut(&mut self.schedules, Schedules::Post, &CoreStage::Last);
            T::add_system(stage);
        }
        self
    }

    pub fn register_topic<T: Resource>(&mut self) -> &mut Self {
        let mut topic_factories = self.world.resource_mut::<TopicFactories>();

        if topic_factories
            .0
            .insert(
                T::type_name().to_string(),
                TopicFactory {
                    factory: zmaxion_topics::topic_factory::<T>,
                },
            )
            .is_none()
        {
            let handlers = self.world.resource::<TopicHandlers>();
            let schedule = &mut self.schedules[Schedules::Post as usize];
            for handler in &handlers.0 {
                match handler {
                    TopicHandler::System => {
                        schedule
                            .add_system_to_stage(CoreStage::Last, SystemTopic::<T>::update_system);
                    }
                    TopicHandler::Async => {
                        schedule
                            .add_system_to_stage(CoreStage::Last, AsyncTopic::<T>::update_system);
                    }
                }
            }
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
            .stage(StartupStage::Startup, |schedule: &mut Schedule| {
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

    pub fn register_pipe<P, Out, Params, Fut, Marker, Rm>(
        &mut self,
        mut pipe: P,
        declaration: PipeDeclaration<Rm>,
    ) -> &mut Self
    where
        P: IntoPipeFactory<Out, Params, Fut, Marker, Rm>,
        Rm: RuntimeMarker,
    {
        let mut definitions = self.world.resource_mut::<PipeDefinitions>();
        let name = pipe.name();
        if definitions.0.contains_key(&name) {
            panic!("Pipe `{}` already registered", name);
        }
        let factory = pipe.into_pipe_factory(declaration);
        definitions
            .0
            .insert(name.into(), PipeDefinition { factory });
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
            .stage(StartupStage::Startup, |schedule: &mut Schedule| {
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

fn stage_mut<'a>(
    schedules: &'a mut [Schedule; 3],
    schedule: Schedules,
    stage_label: &dyn StageLabel,
) -> &'a mut SystemStage {
    schedules[schedule as usize]
        .get_stage_mut(&CoreStage::Last)
        .expect_with(|| format!("{:?} doesn't have {:?}", schedule, stage_label))
}
