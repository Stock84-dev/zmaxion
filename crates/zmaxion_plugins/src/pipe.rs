use std::sync::atomic::AtomicBool;

use bevy_hierarchy::BuildChildren;
use zmaxion_app::prelude::*;
use zmaxion_core::{
    components::{PipeInitializing, ShouldDespawn, TopicState},
    messages::{
        AddSystem, DespawnPipe, DespawnTopic, LoadPipeState, PipeSpawned, PipeSpawning, SpawnPipe,
    },
    prelude::*,
    read_all,
    resources::{PipeNameToEntity, Reschedule, Topics, WorldArc},
    smallvec::SmallVec,
};
use zmaxion_param::{prelude::Errors, PipeFactoryArgs};
use zmaxion_pipe::{
    components::{PipeTask, SystemData},
    resources::PipeDefinitions,
    Pipe,
};
use zmaxion_rt::SpawnFutureExt;
use zmaxion_topic::prelude::*;
use zmaxion_utils::prelude::*;

use crate::error::{PipeSpawnError, SpawnError};

pub struct PipePlugin;

impl Plugin for PipePlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder
            .insert_resource(PipeDefinitions(Default::default()))
            .insert_resource(PipeNameToEntity(Default::default()))
            .add_system_topic::<SpawnPipe>()
            .add_system_topic::<AddSystem>()
            .add_system_topic::<DespawnPipe>()
            .add_system_topic::<PipeSpawning>()
            .add_system_topic::<PipeSpawned>()
            .add_system(spawn_pipe)
            .add_system(poll_spawn_pipe)
            .add_system(pipe_spawning)
            .add_system(despawn_pipe)
    }
}

fn spawn_pipe(
    mut topic_states: Query<&mut TopicState>,
    spawn_pipe: GlobalSystemReader<SpawnPipe>,
    mut defs: ResMut<PipeDefinitions>,
    map: Res<Topics>,
    world: Res<WorldArc>,
    mut commands: Commands,
    errors: Errors,
) {
    for e in &spawn_pipe {
        let mut reader_topics: SmallVec<[Entity; 4]> = Default::default();
        let mut writer_topics: SmallVec<[Entity; 4]> = Default::default();
        let pipe_name = e.0.config.name.as_str();
        let result = defs
            .0
            .get_mut(pipe_name)
            .ok_or_else(|| PipeSpawnError::UnregisteredPipe(pipe_name.to_string()))
            .map_err(|x| SpawnError::Pipe {
                source: x.into(),
                info: e.0.config.clone(),
            });

        let mut def = some_loop!(errors.handle(result));
        let mut i = 0;
        debug!("{:#?}", pipe_name);
        let mut mapper = |_| {
            dbg!(e.0.topic_ids.len(), i, e.0.pipe_id_rel_to_pipeline);
            debug!("key={:#?}", &e.0.topic_ids[e.0.pipe_id_rel_to_pipeline][i]);
            let tmp = map
                .0
                .get(&e.0.topic_ids[e.0.pipe_id_rel_to_pipeline][i])
                .unwrap()
                .id;
            debug!("value={:#?}", tmp);
            i += 1;
            tmp
        };
        reader_topics.extend(e.0.config.reader_topics.iter().map(&mut mapper));
        writer_topics.extend(e.0.config.writer_topics.iter().map(&mut mapper));
        for id in reader_topics.iter().chain(writer_topics.iter()) {
            let mut state: Mut<TopicState> = topic_states.get_mut(*id).unwrap();
            state.n_references += 1;
        }
        debug!("{:#?}", reader_topics);
        debug!("{:#?}", writer_topics);
        let should_despawn = ShouldDespawn(Arc::new(AtomicBool::new(false)));
        let task = def
            .factory
            .new(PipeFactoryArgs {
                world: world.0.clone(),
                config: e.0.clone(),
                should_despawn: should_despawn.0.clone(),
                reader_topics: reader_topics.clone(),
                writer_topics: writer_topics.clone(),
            })
            .spawn();
        commands
            .entity(e.0.pipe_id)
            .insert(should_despawn)
            .insert(PipeTask(task))
            .insert(PipeInitializing(e.0.clone()))
            .insert(Name(e.0.config.name.clone()))
            .insert(SystemData {
                reader_topics,
                writer_topics,
            });
    }
}

fn poll_spawn_pipe(
    mut query: Query<(&mut PipeTask, &PipeInitializing, Entity)>,
    mut spawning: GlobalSystemWriter<PipeSpawning>,
    mut map: ResMut<PipeNameToEntity>,
    mut commands: Commands,
    mut despawn: GlobalSystemWriter<DespawnPipe>,
    errors: Errors,
) {
    for (mut task, e, id) in query.iter_mut() {
        let e: &PipeInitializing = e;
        let result = some_loop!(task.0.poll());
        commands.entity(id).remove::<PipeTask>();
        let adder = match errors.handle(
            result
                .map_err(|x| PipeSpawnError::PipeConstructionFailed {
                    source: x,
                    pipe_name: e.0.config.name.clone(),
                    pipeline: e.0.pipeline.name.clone(),
                })
                .map_err(|x| SpawnError::Pipe {
                    source: x.into(),
                    info: e.0.config.clone(),
                }),
        ) {
            None => {
                despawn.write(DespawnPipe { entity: id });
                continue;
            }
            Some(adder) => adder,
        };
        commands.entity(id).insert(Pipe { adder });

        map.0.insert(e.0.config.name.clone(), id);
        spawning.write(PipeSpawning(e.0.clone()));
        commands.entity(e.0.pipeline_id).push_children(&[id]);
    }
}

fn pipe_spawning(
    mut add: GlobalSystemWriter<AddSystem>,
    mut spawned: GlobalSystemWriter<PipeSpawned>,
    spawning: GlobalSystemReader<PipeSpawning>,
) {
    for e in &spawning {
        debug!("{:#?}", e.0.config.name);
        add.write(AddSystem {
            entity: e.0.pipe_id,
        });
        spawned.write(PipeSpawned(e.0.clone()))
    }
}

fn despawn_pipe(
    despawn_system: GlobalSystemReader<DespawnPipe>,
    mut map: ResMut<PipeNameToEntity>,
    query: Query<(&SystemData, &Name)>,
    mut states: Query<&mut TopicState>,
    mut writer: GlobalSystemWriter<DespawnTopic>,
    mut commands: Commands,
) {
    for e in &despawn_system {
        let (system, name) = query.get(e.entity).unwrap();
        commands.entity(e.entity).despawn();
        map.0.remove(&name.0);
        let mut despawn = |topics: &SmallVec<[Entity; 4]>| {
            for reader in topics {
                let mut state = states.get_mut(*reader).unwrap();
                state.n_references -= 1;
                if state.n_references == 0 {
                    writer.write(DespawnTopic {
                        id: state.id.clone(),
                    });
                }
            }
        };
        despawn(&system.reader_topics);
        despawn(&system.writer_topics);
        commands.insert_resource(Reschedule);
    }
}
