use std::{io::Read, sync::Arc};

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::{
    error::PipeParamError,
    pipe::{factory::PipeFactoryArgs, SpawnPipeInner},
    prelude::{
        bevy_ecs::system::{SystemMeta, SystemParam, SystemParamFetch, SystemParamState},
        *,
    },
    sync::{PrioMutex, PrioMutexGuard},
    topic::{MemTopic, TopicReaderState, TopicWriterState},
};

#[derive(Clone, Copy, Debug)]
pub enum TopicParam {
    Reader(Entity),
    Writer(Entity),
}

#[derive(Clone, Copy, Debug)]
pub enum TopicParamKind {
    None,
    Reader,
    Writer,
}

pub struct ParamBuilder<Args> {
    world: Arc<PrioMutex<World>>,
    //    state: Arc<PipelineState>,
    state: Option<(u64, Vec<u8>)>,
    pipe_id: Entity,
    topic: Option<TopicParam>,
    command: Arc<SpawnPipeInner>,
    arg_i: usize,
    args: Option<AnyResult<Args>>,
    defined_param_kind: TopicParamKind,
}

impl<Args: DeserializeOwned> ParamBuilder<Args> {
    pub fn new<R: Read>(
        factory_args: &PipeFactoryArgs,
        arg_i: usize,
        topic: Option<TopicParam>,
        args: R,
        defined_param_kind: TopicParamKind,
    ) -> Self {
        let args = serde_yaml::from_reader(args);
        ParamBuilder {
            world: factory_args.world.clone(),
            state: None,
            pipe_id: factory_args.config.pipe,
            topic,
            command: factory_args.config.clone(),
            arg_i,
            args: Some(args.map_err(|e| e.into())),
            defined_param_kind,
        }
    }
}

impl<Args: DeserializeOwned> ParamBuilder<Args> {
    pub fn world<'a>(&'a self) -> PrioMutexGuard<'a, World> {
        self.world.lock(0).unwrap()
    }

    pub fn entity(&self) -> Entity {
        self.pipe_id
    }

    pub fn command(&self) -> &Arc<SpawnPipeInner> {
        &self.command
    }

    //    pub fn get_state<T: Resource + Default>(&mut self) -> ZionResult<PipeState<T>> {
    //        // state topic needs to be constructed be get reader with 0, also needs to be first
    // when        // spawning
    //        match &self.state {
    //            None => PipeState {
    //                cur_state_generation: 0,
    //                data: T::default(),
    //                writer:
    // TopicWriter::new(self.world.get::<MemTopic<Stateful<T>>>(id).unwrap()),            },
    //            Some(s) => PipeState {
    //                cur_state_generation: s.0,
    //                data: bincode::deserialize(s.1)?,
    //                writer: (),
    //            },
    //        }
    //    }

    pub fn get_topic_id(&self, reader_name: &str) -> Result<Entity, PipeParamError> {
        Ok(match self.defined_param_kind {
            TopicParamKind::None => {
                return Err(PipeParamError::InvalidPipeParam(
                    self.arg_i,
                    reader_name.into(),
                ))
            }
            TopicParamKind::Reader => match self.topic {
                Some(TopicParam::Reader(id)) => id,
                _ => {
                    return Err(PipeParamError::InvalidPipeParam(
                        self.arg_i,
                        reader_name.into(),
                    ))
                }
            },
            TopicParamKind::Writer => match self.topic {
                Some(TopicParam::Writer(id)) => id,
                _ => {
                    return Err(PipeParamError::InvalidPipeParam(
                        self.arg_i,
                        reader_name.into(),
                    ));
                }
            },
        })
    }

    pub fn args(&mut self) -> AnyResult<Args> {
        self.args.take().unwrap()
    }
}

#[async_trait]
pub trait PipeParamState {
    type Args: DeserializeOwned;
    /// Type of arguments to use when using a constructor
    const KIND: TopicParamKind;
    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized;
    fn should_run(&mut self) -> bool;
    /// This must return some if it doesn't implement default
    fn configure(&self) -> Option<Self>
    where
        Self: Sized;
    fn topic_param(&self) -> Option<TopicParam> {
        None
    }
}

pub trait PipeParam {
    type State: for<'s> PipeParamFetch<'s>;
}

pub trait PipeParamFetch<'s> {
    type Item: PipeParam;
    fn get_param(state: &'s mut Self) -> Self::Item;
}

#[async_trait]
impl PipeParamState for () {
    type Args = ();

    const KIND: TopicParamKind = TopicParamKind::None;

    async fn new(builder: ParamBuilder<()>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        Ok(())
    }

    fn configure(&self) -> Option<Self>
    where
        Self: Sized,
    {
        Some(())
    }

    fn topic_param(&self) -> Option<crate::pipe::TopicParam> {
        None
    }
}

macro_rules! impl_pipe_param (
    ($($param: ident),*) => {
        impl<$($param: PipeParam,)*> PipeParam for ($($param,)*) {
            type Fetch = ($(<$param as PipeParam>::Fetch,)*);
        }
    };
);

macro_rules! impl_pipe_param_fetch (
    ($($state: ident),*) => {
        impl<'s, $($state: PipeParamFetch<'s>),*> PipeParamFetch<'s> for ($($state,)*) {
            type Item = ($(<$state as PipeParamFetch<'s>>::Item,)*);

            fn get_param(state: &'s mut Self) -> Self::Item {
                let ($($state,)*) = state;
                ($($state::get_param($state),)*)
            }
        }
    };
);

macro_rules! impl_build_params {
    ($($param:ident),*) => {
        paste! {
            struct [< P$( $param)* >];
        }
        paste! {
            #[allow(non_snake_case)]
            impl [< P$( $param)* >] {
                async fn build_params<$($param),*>(args: PipeFactoryArgs) -> Result<($(<$param as PipeParam>::Fetch,)*), PipeParamError>
                where
                    $(
                        $param: PipeParam + 'static,
                        <$param as PipeParam>::Fetch: PipeParamState,
                    )*
                {
                    let mut arg_i = 0;
                    let mut reader_i = 0;
                    let mut writer_i = 0;
                    #[allow(unused_mut)]
                    let mut pipe_args = &args.config.config.args[..];

                    let result = futures_util::try_join!(
                        $({
                            let topic_param = match <<$param as PipeParam>::Fetch as PipeParamState>::KIND {
                                TopicParamKind::Reader => {
//                                    debug!("rtopics{:#?}", args.reader_topics);
                                    let id = args.reader_topics.get(reader_i)
                                        .ok_or_else(|| PipeParamError::InvalidPipeParam(
                                            arg_i,
                                            $param::type_name().to_string(),
                                    ))?;
//                                    debug!("reader={:#?}", id);
                                    reader_i += 1;
                                    Some(TopicParam::Reader(*id))
                                }
                                TopicParamKind::Writer => {
//                                    debug!("wtopics{:#?}", args.writer_topics);
                                    let id = args.writer_topics.get(writer_i)
                                        .ok_or_else(|| PipeParamError::InvalidPipeParam(
                                            arg_i,
                                            $param::type_name().to_string(),
                                    ))?;
                                    writer_i += 1;
//                                    debug!("writer={:#?}", id);
                                    Some(TopicParam::Writer(*id))
                                }
                                TopicParamKind::None => None,
                            };
                            let builder = args.to_param_builder(
                                arg_i,
                                topic_param,
                                &mut pipe_args,
                                <<$param as PipeParam>::Fetch as PipeParamState>::KIND,
                            );
                            arg_i += 1;
                            <<$param as PipeParam>::Fetch as PipeParamState>::new(builder)
                        }),*
                    );
                    result.map_err(|e| PipeParamError::Other(e))
                }
            }
        }
    };
}

pub(super) use impl_build_params;
pub(super) use impl_pipe_param;
pub(super) use impl_pipe_param_fetch;

use crate::{
    error::assert_config_provided, pipe::PipeObserver, prelude::bevy_ecs::archetype::Archetype,
};

#[async_trait]
pub trait ParamStateImpl<'w, 's>: 'static {
    type Topic: Component + Clone;
    type Args: DeserializeOwned + Send + 'static;
    type Param: PipeParam<State = Self> + SystemParam<Fetch = Self>;
    const KIND: TopicParamKind;

    fn new_rw(topic: Self::Topic) -> AnyResult<Self>;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self>
    where
        Self: Sized,
    {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world();
        let topic = world.get::<Self::Topic>(id).unwrap();
        Self::new_rw(topic.clone())
    }

    #[inline]
    fn should_run(&mut self) -> bool {
        true
    }

    fn configure(&self) -> Option<Self>
    where
        Self: Sized,
    {
        Some(self.clone())
    }

    fn topic_param(&self) -> Option<TopicParam> {
        match Self::KIND {
            TopicParamKind::None => None,
            TopicParamKind::Reader => Some(TopicParam::Reader(Entity::from_raw(u32::MAX))),
            TopicParamKind::Writer => Some(TopicParam::Writer(Entity::from_raw(u32::MAX))),
        }
    }

    fn init(world: &mut World, system_meta: &mut SystemMeta, config: Option<Self>) -> Self
    where
        Self: Sized,
    {
        assert_config_provided(config)
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
    }

    fn apply(&mut self, world: &mut World) {
    }

    fn default_config() -> Option<Self> {
        None
    }

    unsafe fn system_get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Param {
        Self::get_param(state)
    }

    fn pipe_get_param(state: &'s mut Self) -> Self::Param {
        Self::get_param(state)
    }

    fn get_param(&'s mut self) -> Self::Param;

    /// Called before the pipe gets executed
    fn pipe_executing(&mut self) {
    }
    /// Called after the pipe has executed
    fn pipe_executed(&mut self) {
    }
}

pub trait ParamImpl {
    type State: for<'s> PipeParamState + SystemParamState + PipeParamFetch<'s> + PipeObserver;
}

#[macro_export]
macro_rules! impl_param {
    ($param:ident) => {
        impl<T> PipeParam for $param {
            type Fetch = <$param<T> as $crate::ParamImpl>;
        }

        impl<T> SystemParam for $param {
            type Fetch = <$param<T> as $crate::ParamImpl>;
        }
    }
}

#[doc(hidden)]
pub mod __async_trait__ {
    pub use async_trait::async_trait;
}

#[macro_export]
macro_rules! impl_state_param {
    ($state:ident) => {
        #[$crate::pipe::param::__async_trait__::async_trait]
        impl<T: Send + Sync + 'static> $crate::pipe::param::PipeParamState for $state<T> {
            type Args = <$state<T> as $crate::pipe::param::ParamStateImpl<'static, 'static>>::Args;

            const KIND: $crate::pipe::param::TopicParamKind =
                <$state<T> as $crate::ParamStateImpl>::KIND;

            async fn new(
                builder: $crate::pipe::param::ParamBuilder<Self::Args>,
            ) -> AnyResult<Self>
            where
                Self: Sized,
            {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::new(builder).await
            }

            fn configure(&self) -> Option<Self>
            where
                Self: Sized,
            {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::configure(self)
            }
        }

        unsafe impl<T: Send + Sync + 'static> $crate::bevy::ecs::system::SystemParamState
            for $state<T>
        {
            type Config = Option<Self>;

            fn init(
                world: &mut $crate::prelude::World,
                system_meta: &mut $crate::bevy::ecs::system::SystemMeta,
                config: Self::Config,
            ) -> Self {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::init(world, system_meta, config)
            }

            fn new_archetype(
                &mut self,
                archetype: &$crate::bevy::ecs::archetype::Archetype,
                system_meta: &mut $crate::bevy::ecs::system::SystemMeta,
            ) {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::new_archetype(self, archetype, system_meta);
            }

            fn apply(&mut self, world: &mut $crate::bevy::prelude::World) {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::apply(self, world);
            }

            fn default_config() -> Self::Config {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::default_config()
            }
        }

        impl<'w, 's, T: Send + Sync + 'static>
            $crate::bevy::ecs::system::SystemParamFetch<'w, 's> for $state<T>
        {
            type Item = <$state<T> as $crate::pipe::param::ParamStateImpl<'w, 's>>::Param;

            unsafe fn get_param(
                state: &'s mut Self,
                system_meta: &$crate::bevy::ecs::system::SystemMeta,
                world: &'w $crate::bevy::prelude::World,
                change_tick: u32,
            ) -> Self::Item {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::system_get_param(
                    state,
                    system_meta,
                    world,
                    change_tick,
                )
            }
        }

        impl<'s, T: Send + Sync + 'static> $crate::pipe::param::PipeParamFetch<'s>
            for $state<T>
        {
            type Item = <$state<T> as $crate::pipe::param::ParamStateImpl<'static, 's>>::Param;

            fn get_param(state: &'s mut Self) -> Self::Item {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::pipe_get_param(state)
            }
        }

        impl<T: Send + Sync + 'static> $crate::::pipe::PipeObserver for $state<T> {
            fn pipe_executing(&mut self) {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::pipe_executing(self)
            }

            fn pipe_executed(&mut self) {
                <$state<T> as $crate::pipe::param::ParamStateImpl>::pipe_executed(self)
            }
        }
    };
}
