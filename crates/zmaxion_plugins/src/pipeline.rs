use std::sync::Arc;

use bevy_ecs::system::{SystemMeta, SystemParamState, SystemState};
use bevy_hierarchy::Children;
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
    resources::LogErrorsSynchronously,
};
use zmaxion_param::{
    prelude::{ErrorEvent, ErrorsState},
    AsyncReaderState, AsyncTopic, AsyncWriterState,
};
use zmaxion_topic::prelude::*;
use zmaxion_utils::prelude::*;

pub struct PipelinePlugin;

impl Plugin for PipelinePlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder
            .add_system_topic::<SpawnPipeline>()
            .add_system_topic::<PipelineSpawning>()
            .add_system_topic::<PipelineSpawned>()
            .add_system_topic::<DespawnPipeline>()
            .add_system(relay_pipeline_events_0)
            .add_system(relay_pipeline_events_1)
    }
}

fn relay_pipeline_events_0(
    read_spawn_pipeline: GlobalSystemReader<SpawnPipeline>,
    read_pipeline_spawned: GlobalSystemReader<PipelineSpawned>,
    pipelines: Query<&Pipeline>,
    mut errors: GlobalAsyncReader<ErrorEvent>,
    mut topics_spawner: GlobalSystemWriter<SpawnTopics>,
    mut write_pipeline_spawning: GlobalSystemWriter<PipelineSpawning>,
    mut write_despawn_pipeline: GlobalSystemWriter<DespawnPipeline>,
    mut commands: Commands,
) {
    // command -> id -> ing
    // ing -> add kafka -> spawned
    // spawned -> spawn system and topic
    write_pipeline_spawning.extend(read_spawn_pipeline.iter().map(|e| {
        let errors_topic = AsyncTopic::<ErrorEvent>::default();
        let reader = AsyncReaderState::from(errors_topic.clone());
        let pipeline_id = commands
            .spawn()
            .insert(Pipeline)
            .insert(errors_topic)
            .insert(reader)
            .insert(Generation(0))
            .insert(Name(e.0.name.clone()))
            .id();
        let mut inner = Arc::as_ref(&e.0).clone();
        // add a state topic where we store pipeline state
        //        let state_topic_name = e.0.name.clone() + "_state";
        // make sure that pipes can read/write to it
        //        for pipe in &mut inner.pipes {
        //            let pipe = Arc::make_mut(pipe);
        //            pipe.reader_topics.push(state_topic_name.clone());
        //            pipe.writer_topics.push(state_topic_name.clone());
        //            // don't know that this does
        //            //                let topic = format!("{}:{}", e.0.id, pipe.name);
        //            //                Arc::make_mut(pipe).writer_topics.push(topic.clone());
        //            //                inner.topics.push(Arc::new(TopicConfig {
        //            //                    name: topic,
        //            //                    connector: "kafka".to_string(),
        //            //                    schema: "".to_string(),
        //            //                    initial_message_set: vec![],
        //            //                    n_initial_message_sets: 0,
        //            //                    args: vec![],
        //            //                    lifetime: TopicLifetime::Global,
        //            //                    access: TopicAccess::Private,
        //            //                    idempotence: Idempotence::No,
        //            //                    async_support: AsyncSupport::No,
        //            //                    transactional: TransactionSupport::No,
        //            //                }));
        //        }
        //        inner.topics.push(Arc::new(TopicConfig {
        //            name: state_topic_name,
        //            connector: inner.state_connector.clone(),
        //            schema: PipelineState::type_name().into(),
        //            initial_message_set: vec![],
        //            n_initial_message_sets: 0,
        //            args: vec![],
        //            lifetime: TopicLifetime::Pipeline(e.0.name.clone()),
        //            access: TopicAccess::Private,
        //            idempotence: Idempotence::No,
        //            async_support: AsyncSupport::No,
        //            transactional: TransactionSupport::No,
        //        }));
        PipelineSpawning {
            id: pipeline_id,
            data: Arc::new(inner),
        }
    }));
    write_despawn_pipeline.extend(errors.try_read().iter().flat_map(|x| x.iter()).filter_map(
        |e| {
            pipelines
                .get(e.pipeline_id)
                .ok()
                .map(|_| DespawnPipeline { id: e.pipeline_id })
        },
    ));
    topics_spawner.extend(read_pipeline_spawned.iter().map(|e| SpawnTopics {
        pipeline: e.data.clone(),
        pipeline_id: e.id,
    }));
}

fn relay_pipeline_events_1(
    read_pipeline_spawning: GlobalSystemReader<PipelineSpawning>,
    read_despawn_pipeline: GlobalSystemReader<DespawnPipeline>,
    children: Query<&Children>,
    mut write_pipeline_spawned: GlobalSystemWriter<PipelineSpawned>,
    mut despawn_pipe: GlobalSystemWriter<DespawnPipe>,
    mut commands: Commands,
) {
    despawn_pipe.extend(
        read_despawn_pipeline
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
    write_pipeline_spawned.extend(read_pipeline_spawning.iter().map(|e| PipelineSpawned {
        id: e.id,
        data: e.data.clone(),
    }));
}
