use std::{borrow::Cow, sync::Arc};

use zmaxion_app::prelude::*;
use zmaxion_core::{
    error::{Errors, SpawnError, TopicSpawnError},
    models::TopicAccess,
    pipe::{messages::SpawnPipe, SpawnPipeInner},
    pipeline::components::PipeIdRelToPipeline,
    prelude::*,
    topic::{
        components::{ResTopicReader, ResTopicWriter, TopicState},
        messages::{
            DespawnTopic, SpawnTopicInner, SpawnTopics, TopicSpawned, TopicSpawning, TopicsSpawned,
            TopicsSpawning,
        },
        resources::{TopicDefinitions, Topics},
        IoMode, Topic, TopicId, TopicSpawnerArgs,
    },
};

pub struct TopicPlugin;

impl Plugin for TopicPlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder
            .insert_resource(Topics(Default::default()))
            .insert_resource(TopicDefinitions(Default::default()))
            .add_system_topic::<SpawnTopics>()
            .add_system_topic::<TopicsSpawning>()
            .add_system_topic::<TopicsSpawned>()
            .add_system_topic::<TopicSpawning>()
            .add_system_topic::<TopicSpawned>()
            .add_system_topic::<DespawnTopic>()
            .add_system(spawn_topics)
            .add_system(despawn_topics)
            .add_system(relay_topic_events)
    }
}

fn spawn_topics(
    reader: ResTopicReader<SpawnTopics>,
    writer: ResTopicWriter<TopicSpawning>,
    mut topics: ResMut<Topics>,
    topics_spawning: ResTopicWriter<TopicsSpawning>,
    global: Res<GlobalEntity>,
    definitions: Res<TopicDefinitions>,
    errors: Errors,
    mut commands: Commands,
) {
    for e in read_all!(reader) {
        let mut topic_ids = Vec::new();
        for pipe in &e.pipeline.pipes {
            let mut ids = Vec::new();
            let pipe_state_topic = format!("{}:{}", e.pipeline.id, pipe.name);
            for (topic_name, is_reader) in pipe.reader_topics.iter().map(|x| (x, true)).chain(
                pipe.writer_topics
                    .iter()
                    //                    .chain(iter::once(&pipe_state_topic))
                    .map(|x| (x, false)),
            ) {
                let result = e
                    .pipeline
                    .topics
                    .iter()
                    .find(|t| &t.name == topic_name)
                    .some()
                    .map_err(|_| TopicSpawnError::MissingTopicConfig(topic_name.clone()))
                    .map_err(|x| SpawnError::Topic {
                        source: x.into(),
                        info: None,
                    });
                let config = some_loop!(errors.handle_with(result, e.pipeline_id));
                let mut io_mode = IoMode::Write;
                if config.connector == "" {
                    io_mode = IoMode::ReadWrite;
                } else if is_reader {
                    io_mode = IoMode::Read;
                }
                let topic_id = TopicId {
                    name: Cow::Owned(config.name.clone()),
                    io_mode,
                    pipeline_id: match config.access {
                        TopicAccess::Private => e.pipeline_id,
                        TopicAccess::Public => global.0,
                    },
                };
                ids.push(topic_id.clone());

                match topics.0.get_mut(&topic_id) {
                    None => {
                        let id = commands.spawn().id();
                        topics.0.entry(topic_id.clone()).or_insert(Topic {
                            id,
                            n_references: 1,
                        });
                        let result = definitions
                            .0
                            .get(&config.schema)
                            .some()
                            .map_err(|_| TopicSpawnError::UnregisteredTopic(config.schema.clone()))
                            .map_err(|x| SpawnError::Topic {
                                source: x.into(),
                                info: None,
                            });
                        let definition = some_loop!(errors.handle_with(result, e.pipeline_id));
                        (definition.spawner)(&mut TopicSpawnerArgs {
                            commands: commands.entity(id),
                            is_reader: false,
                        });
                        commands.entity(id).insert(TopicState {
                            id: topic_id.clone(),
                            n_references: 0,
                        });
                        info!("{:#?}", config.name);
                        info!("{:#?}", id);
                        writer.write(TopicSpawning {
                            topic_id,
                            id: id.clone(),
                            data: Arc::new(SpawnTopicInner {
                                pipeline: e.pipeline.clone(),
                                pipeline_id: e.pipeline_id,
                                config: config.clone(),
                                is_reader,
                            }),
                        });
                    }
                    Some(topic) => {
                        topic.n_references += 1;
                    }
                }
            }
            topic_ids.push(ids);
        }
        debug!("{:#?}", topic_ids);
        topics_spawning.write(TopicsSpawning {
            topic_ids: Arc::new(topic_ids),
            pipeline: e.pipeline.clone(),
            pipeline_id: e.pipeline_id,
        })
    }
}

fn despawn_topics(
    topic_commands: ResTopicReader<DespawnTopic>,
    mut map: ResMut<Topics>,
    mut commands: Commands,
) {
    for command in read_all!(topic_commands) {
        let entity = map.0.remove(&command.id).unwrap();
        commands.entity(entity.id).despawn();
    }
}

fn relay_topic_events(
    topic_spawning: ResTopicReader<TopicSpawning>,
    topic_spawned: ResTopicWriter<TopicSpawned>,
    topics_spawning: ResTopicReader<TopicsSpawning>,
    topics_spawned: ResTopicWriter<TopicsSpawned>,
    topics_spawned_read: ResTopicReader<TopicsSpawned>,
    spawn_pipe: ResTopicWriter<SpawnPipe>,
    mut commands: Commands,
) {
    if let Some(mut t) = topic_spawning.try_read() {
        for e in t.read_all() {
            topic_spawned.write(TopicSpawned {
                id: e.id,
                data: e.data.clone(),
            });
        }
    }
    if let Some(mut t) = topics_spawning.try_read() {
        for e in t.read_all() {
            topics_spawned.write(TopicsSpawned {
                topic_ids: e.topic_ids.clone(),
                pipeline: e.pipeline.clone(),
                pipeline_id: e.pipeline_id,
            });
        }
    }
    if let Some(mut t) = topics_spawned_read.try_read() {
        for e in t.read_all() {
            for (pipe_i, pipe) in e.pipeline.pipes.iter().enumerate() {
                let pipe_id = commands.spawn().insert(PipeIdRelToPipeline(pipe_i)).id();

                spawn_pipe.write(SpawnPipe(Arc::new(SpawnPipeInner {
                    pipeline_id: e.pipeline_id,
                    pipe: pipe_id,
                    config: pipe.clone(),
                    pipeline: e.pipeline.clone(),
                    topic_ids: e.topic_ids.clone(),
                    pipe_id_rel_to_pipeline: pipe_i,
                    state_generation: e.pipeline.state_generation,
                })));
            }
        }
    }
}
