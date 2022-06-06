use std::sync::Arc;

use bevy::ecs::system::{SystemMeta, SystemParamState, SystemState};
use zmaxion_app::prelude::*;
use zmaxion_core::{
    components::{Generation, Pipeline, PipelineState},
    messages::{
        DespawnPipe, DespawnPipeline, PipelineSpawned, PipelineSpawning, SpawnPipeline, SpawnTopics,
    },
    models::config::{
        AsyncSupport, Idempotence, TopicAccess, TopicConfig, TopicLifetime, TransactionSupport,
    },
    prelude::*,
};

pub struct PipelinePlugin;

impl Plugin for PipelinePlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder
            .add_system_topic::<SpawnPipeline>()
            .add_system_topic::<PipelineSpawning>()
            .add_system_topic::<PipelineSpawned>()
            .add_system_topic::<DespawnPipeline>()
            .add_system(relay_pipeline_events)
    }
}

fn relay_pipeline_events(
    read_pipeline_spawned: GlobalSystemReader<PipelineSpawned>,
    topics_spawner: GlobalSystemWriter<SpawnTopics>,
    read_pipeline_spawning: GlobalSystemReader<PipelineSpawning>,
    write_pipeline_spawned: GlobalSystemWriter<PipelineSpawned>,
    read_spawn_pipeline: GlobalSystemReader<SpawnPipeline>,
    write_pipeline_spawning: GlobalSystemWriter<PipelineSpawning>,
    errors: GlobalSystemReader<ErrorEvent>,
    write_despawn_pipeline: GlobalSystemWriter<DespawnPipeline>,
    read_despawn_pipeline: GlobalSystemReader<DespawnPipeline>,
    despawn_pipe: GlobalSystemWriter<DespawnPipe>,
    pipelines: Query<&Pipeline>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    // command -> id -> ing
    // ing -> add kafka -> spawned
    // spawned -> spawn system and topic
    if let Some(mut t) = read_spawn_pipeline.try_read() {
        write_pipeline_spawning.write_all(t.read_all().iter().map(|e| {
            let errors_topic = MemTopic::<ErrorEvent>::new();
            let errors_topic_reader_state = TopicReaderState::from(errors_topic.clone());
            let workflow_id = commands
                .spawn()
                .insert(Pipeline)
                .insert(errors_topic)
                .insert(errors_topic_reader_state)
                .insert(Generation(0))
                .insert(Name(e.0.name.clone()))
                .id();
            let mut inner = Arc::as_ref(&e.0).clone();
            // add a state topic where we store pipeline state
            let state_topic_name = e.0.name.clone() + "_state";
            // make sure that pipes can read/write to it
            for pipe in &mut inner.pipes {
                let pipe = Arc::make_mut(pipe);
                pipe.reader_topics.push(state_topic_name.clone());
                pipe.writer_topics.push(state_topic_name.clone());
                // don't know that this does
                //                let topic = format!("{}:{}", e.0.id, pipe.name);
                //                Arc::make_mut(pipe).writer_topics.push(topic.clone());
                //                inner.topics.push(Arc::new(TopicConfig {
                //                    name: topic,
                //                    connector: "kafka".to_string(),
                //                    schema: "".to_string(),
                //                    initial_message_set: vec![],
                //                    n_initial_message_sets: 0,
                //                    args: vec![],
                //                    lifetime: TopicLifetime::Global,
                //                    access: TopicAccess::Private,
                //                    idempotence: Idempotence::No,
                //                    async_support: AsyncSupport::No,
                //                    transactional: TransactionSupport::No,
                //                }));
            }
            inner.topics.push(Arc::new(TopicConfig {
                name: state_topic_name,
                connector: inner.state_connector.clone(),
                schema: PipelineState::type_name().into(),
                initial_message_set: vec![],
                n_initial_message_sets: 0,
                args: vec![],
                lifetime: TopicLifetime::Pipeline(e.0.name.clone()),
                access: TopicAccess::Private,
                idempotence: Idempotence::No,
                async_support: AsyncSupport::No,
                transactional: TransactionSupport::No,
            }));
            PipelineSpawning {
                id: workflow_id,
                data: Arc::new(inner),
            }
        }));
    }
    if let Some(mut t) = read_pipeline_spawning.try_read() {
        write_pipeline_spawned.write_all(t.read_all().iter().map(|e| PipelineSpawned {
            id: e.id,
            data: e.data.clone(),
        }));
    }
    if let Some(mut t) = read_pipeline_spawned.try_read() {
        topics_spawner.write_all(t.read_all().iter().map(|e| SpawnTopics {
            pipeline: e.data.clone(),
            pipeline_id: e.id,
        }));
    }
    if let Some(mut t) = errors.try_read() {
        write_despawn_pipeline.write_all(t.read_all().iter().filter_map(|e| {
            pipelines
                .get(e.pipeline_id)
                .ok()
                .map(|_| DespawnPipeline { id: e.pipeline_id })
        }));
    }
    if let Some(mut t) = read_despawn_pipeline.try_read() {
        despawn_pipe.write_all(
            t.read_all()
                .iter()
                .filter_map(|e| {
                    commands.entity(e.id).despawn();
                    children
                        .get(e.id)
                        .ok()
                        .map(|x| x.iter().map(|x| DespawnPipe { entity: *x }))
                })
                .flatten(),
        );
    }
}
