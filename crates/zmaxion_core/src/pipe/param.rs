use crate::error::PipeParamError;
use crate::pipe::factory::PipeFactoryArgs;
use crate::pipe::SpawnPipeInner;
use crate::prelude::*;
use crate::sync::{PrioMutex, PrioMutexGuard};
use crate::topic::{MemTopic, TopicReaderState, TopicWriterState};
use async_trait::async_trait;
use bevy::ecs::system::Resource;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::sync::Arc;

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
}

impl<Args: DeserializeOwned> ParamBuilder<Args> {
    pub fn new<R: Read>(
        factory_args: &PipeFactoryArgs,
        arg_i: usize,
        topic: Option<TopicParam>,
        args: R,
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

    pub fn get_reader<T: Resource>(&self) -> Result<TopicReaderState<T>, PipeParamError> {
        //        debug!("{:#?}", self.config.name);
        //        debug!("{:#?}", self.topic);
        let id = match self.topic {
            Some(TopicParam::Reader(id)) => id,
            _ => {
                return Err(PipeParamError::InvalidPipeParam(
                    self.arg_i,
                    TopicReader::<T>::type_name().into(),
                ))
            }
        };
        let world = self.world.lock(0).unwrap();
        //        error!("{:#?}", id);
        //        error!("{:#?}", T::id_name());
        let topic = world.get::<MemTopic<T>>(id).unwrap();
        Ok(TopicReaderState::from(topic.clone()))
    }

    pub fn get_writer<T: Resource>(&self) -> Result<TopicWriterState<T>, PipeParamError> {
        //        debug!("{:#?}", self.topic);
        let id = match self.topic {
            Some(TopicParam::Writer(id)) => id,
            _ => {
                return Err(PipeParamError::InvalidPipeParam(
                    self.arg_i,
                    TopicWriter::<T>::type_name().into(),
                ));
            }
        };
        //        error!("{:#?}", id);
        //        error!("{:#?}", T::id_name());
        let world = self.world.lock(0).unwrap();
        let topic = world.get::<MemTopic<T>>(id).unwrap();
        Ok(TopicWriterState::from(topic.clone()))
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
    /// This must return some if it doesn't implement default
    fn configure(&self) -> Option<Self>
    where
        Self: Sized;
    fn topic_param(&self) -> Option<TopicParam> {
        None
    }
}

pub trait PipeParam {
    type Fetch: for<'s> PipeParamFetch<'s>;
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
                                &mut pipe_args
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
