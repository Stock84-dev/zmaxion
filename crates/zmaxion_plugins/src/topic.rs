use std::{borrow::Cow, sync::Arc};

use zmaxion_app::prelude::*;
use zmaxion_core::{
    components::{PipeIdRelToPipeline, TopicState},
    messages::{
        DespawnTopic, SpawnPipe, SpawnTopicInner, SpawnTopics, TopicSpawned, TopicSpawning,
        TopicsSpawned, TopicsSpawning,
    },
    models::{
        config::TopicAccess, IoMode, SpawnPipeInner, TopicHandler, TopicId, TopicRef,
        TopicSpawnerArgs,
    },
    prelude::*,
    resources::{TopicFactories, TopicHandlers, Topics},
};
use zmaxion_flow::{
    default::{DefaultMessage, DefaultPool, DefaultPoolFetcher},
    flow::ToEventWriter,
    flower::ToEventWriterCombo,
    prelude::*,
};
use zmaxion_param::prelude::Errors;
use zmaxion_topic::{
    prelude::{GenericGlobalBevyWriter, GlobalBevyReader, GlobalBevyWriter},
    system_df_topic::BevyFlower,
};
use zmaxion_utils::prelude::*;

use crate::error::{SpawnError, TopicSpawnError};

pub struct TopicPlugin(pub HashSet<TopicHandler>);

impl Default for TopicPlugin {
    fn default() -> Self {
        let mut set = HashSet::new();
        set.extend(TopicHandler::variants());
        Self(set)
    }
}

impl Plugin for TopicPlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder
        //        builder.insert_resource(TopicHandlers(Default::default()));
        //        for handler in self.0 {
        //            builder.add_topic_handler(handler);
        //        }
        //        builder
        //            .insert_resource(Topics(Default::default()))
        //            .insert_resource(TopicFactories(Default::default()))
        //            .add_system_topic::<SpawnTopics>()
        //            .add_system_topic::<TopicsSpawning>()
        //            .add_system_topic::<TopicsSpawned>()
        //            .add_system_topic::<TopicSpawning>()
        //            .add_system_topic::<TopicSpawned>()
        //            .add_system_topic::<DespawnTopic>()
        //            .add_system(spawn_topics)
        //            .add_system(despawn_topics)
        //            .add_system(relay_topic_events_0)
        //            .add_system(relay_topic_events_1)
    }
}
// fn spawn_topics(
//    reader: GlobalBevyReader<SpawnTopics>,
//    mut writer: GlobalBevyWriter<TopicSpawning>,
//    mut topics: ResMut<Topics>,
//    mut topics_spawning: GlobalBevyWriter<TopicsSpawning>,
//    global: Res<GlobalEntity>,
//    factories: Res<TopicFactories>,
//    handlers: Res<TopicHandlers>,
//    errors: Errors,
//    mut commands: Commands,
//) {
//    for e in &reader {
//        let mut topic_ids = Vec::new();
//        for pipe in &e.pipeline.pipes {
//            let mut ids = Vec::new();
//            let pipe_state_topic = format!("{}:{}", e.pipeline.id, pipe.name);
//            for (topic_name, is_reader) in pipe.reader_topics.iter().map(|x| (x, true)).chain(
//                pipe.writer_topics
//                    .iter()
//                    //                    .chain(iter::once(&pipe_state_topic))
//                    .map(|x| (x, false)),
//            ) {
//                let result = e
//                    .pipeline
//                    .topics
//                    .iter()
//                    .find(|t| &t.name == topic_name)
//                    .ok_or_else(|| TopicSpawnError::MissingTopicConfig(topic_name.clone()))
//                    .map_err(|x| SpawnError::Topic {
//                        source: x.into(),
//                        info: None,
//                    });
//                let config = some_loop!(errors.handle_with(result, e.pipeline_id));
//                let mut io_mode = IoMode::Write;
//                if config.connector == "" {
//                    io_mode = IoMode::ReadWrite;
//                } else if is_reader {
//                    io_mode = IoMode::Read;
//                }
//                let topic_id = TopicId {
//                    name: Cow::Owned(config.name.clone()),
//                    io_mode,
//                    pipeline_id: match config.access {
//                        TopicAccess::Private => e.pipeline_id,
//                        TopicAccess::Public => global.0,
//                    },
//                };
//                ids.push(topic_id.clone());
//
//                match topics.0.get_mut(&topic_id) {
//                    None => {
//                        let id = commands.spawn().id();
//                        topics.0.entry(topic_id.clone()).or_insert(TopicRef {
//                            id,
//                            n_references: 1,
//                        });
//                        let result = factories
//                            .0
//                            .get(&config.schema)
//                            .ok_or_else(|| {
//                                TopicSpawnError::UnregisteredTopic(config.schema.clone())
//                            })
//                            .map_err(|x| SpawnError::Topic {
//                                source: x.into(),
//                                info: None,
//                            });
//                        let factory = some_loop!(errors.handle_with(result, e.pipeline_id));
//                        (factory.factory)(&mut TopicSpawnerArgs {
//                            commands: commands.entity(id),
//                            is_reader: false,
//                            handlers: &handlers.0,
//                        });
//                        commands.entity(id).insert(TopicState {
//                            id: topic_id.clone(),
//                            n_references: 0,
//                        });
//                        info!("{:#?}", config.name);
//                        info!("{:#?}", id);
//                        writer.write(TopicSpawning {
//                            topic_id,
//                            id: id.clone(),
//                            data: Arc::new(SpawnTopicInner {
//                                pipeline: e.pipeline.clone(),
//                                pipeline_id: e.pipeline_id,
//                                config: config.clone(),
//                                is_reader,
//                            }),
//                        });
//                    }
//                    Some(topic) => {
//                        topic.n_references += 1;
//                    }
//                }
//            }
//            topic_ids.push(ids);
//        }
//        debug!("{:#?}", topic_ids);
//        topics_spawning.write(TopicsSpawning {
//            topic_ids: Arc::new(topic_ids),
//            pipeline: e.pipeline.clone(),
//            pipeline_id: e.pipeline_id,
//        })
//    }
//}
// fn despawn_topics(
//    topic_commands: GlobalBevyReader<DespawnTopic>,
//    mut map: ResMut<Topics>,
//    mut commands: Commands,
//) {
//    for command in &topic_commands {
//        let entity = map.0.remove(&command.id).unwrap();
//        commands.entity(entity.id).despawn();
//    }
//}
// fn relay_topic_events_0(
//    topic_spawning: GlobalBevyReader<TopicSpawning>,
//    mut topic_spawned: GlobalBevyWriter<TopicSpawned>,
//    topics_spawning: GlobalBevyReader<TopicsSpawning>,
//    mut topics_spawned: GlobalBevyWriter<TopicsSpawned>,
//) {
//    Flow::from(topic_spawning)
//        .map(|x| TopicSpawned {
//            id: x.id,
//            data: x.data.clone(),
//        })
//        .write(topic_spawned)
//        .block();
//    let a = topic_spawning.flow();
//    a
//        //        .map(|x| TopicSpawned {
//        //            id: x.id,
//        //            data: x.data.clone(),
//        //        })
//        .write(topic_spawned)
//        .run()?;
//
//    topic_spawning.flow().map(|x| TopicSpawned {
//        id: x.id,
//        data: x.data.clone(),
//    });
//    topic_spawned.extend(topic_spawning.iter().map(|e| TopicSpawned {
//        id: e.id,
//        data: e.data.clone(),
//    }));
//    topics_spawned.extend(topics_spawning.iter().map(|e| TopicsSpawned {
//        topic_ids: e.topic_ids.clone(),
//        pipeline: e.pipeline.clone(),
//        pipeline_id: e.pipeline_id,
//    }));
//}
// fn relay_topic_events_1(
//    mut topics_spawned_read: GlobalBevyReader<TopicsSpawned>,
//    mut spawn_pipe: GlobalBevyWriter<SpawnPipe>,
//    mut commands: Commands,
//) {
//    for e in &topics_spawned_read {
//        for (pipe_i, pipe) in e.pipeline.pipes.iter().enumerate() {
//            let pipe_id = commands.spawn().insert(PipeIdRelToPipeline(pipe_i)).id();
//
//            spawn_pipe.write(SpawnPipe(Arc::new(SpawnPipeInner {
//                pipeline_id: e.pipeline_id,
//                pipe_id,
//                config: pipe.clone(),
//                pipeline: e.pipeline.clone(),
//                topic_ids: e.topic_ids.clone(),
//                pipe_id_rel_to_pipeline: pipe_i,
//                state_generation: e.pipeline.state_generation,
//            })));
//        }
//    }
//}
// fn b() {
//    unsafe {
//        a::<
//            BevyFlower<DefaultPoolFetcher, TopicsSpawning>,
//            DefaultPool<TopicsSpawning>,
//            GlobalBevyWriter<TopicsSpawning>, /*
// GenericGlobalBevyWriter<DefaultPool<TopicsSpawning>, TopicsSpawning>, */
//        >(std::mem::uninitialized());
//    }
//}
// fn a<'s, I, P, W>(w: W)
// where
//    I: Flower<'s>,
//    W: ToEventWriterCombo<'s, I, P>,
//{
//}
//
//// pub trait ToEventWriterCombo<'a, I: Flower<'a>, P>:
//// ToAsyncEventWriter<P, <<I::Item as Message<'a>>::PooledMessage as Message<'a>>::Value>
//// + ToEventWriter<
////    <<I as Flower<'a>>::Item as Message<'a>>::Pool,
////    <<<I as Flower<'a>>::Item as Message<'a>>::PooledMessage as Message<'a>>::Value,
//// >
//// {
//// }
