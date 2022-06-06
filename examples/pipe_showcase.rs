use std::sync::{atomic::Ordering, Arc};

use zmaxion::prelude::*;
use zmaxion_internal::{
    core::{
        bevy::{ecs::system::SystemState, log::LogPlugin},
        models::{
            AsyncSupport, Idempotence, TopicAccess, TopicConfig, TopicLifetime, TransactionSupport,
        },
        pipe::PipeConfig,
        pipeline::{messages::SpawnPipeline, SpawnPipelineInner},
        resources::LogErrorsSynchronously,
        smallvec::smallvec,
        state::components::PipelineState,
    },
    plugins::LogErrorsPlugin,
    rt::tokio,
};

pub struct A(String);
pub struct B(String);

fn relay(reader: TopicReader<B>, writer: TopicWriter<B>) {
    writer.write_all(
        read_all!(reader)
            .iter()
            .map(|x| B(x.0.to_owned() + " world")),
    );
}

fn transmitter(writer: TopicWriter<A>, reader: TopicReader<B>) {
    writer.write(A("hello".into()));
    for e in read_all!(reader) {
        debug!("{:#?}", e.0);
    }
}

pub async fn async_transmitter(writer: TopicWriter<'static, A>, reader: TopicReader<'static, B>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    writer.write(A("hello".into()));
    for e in read_all!(reader) {
        debug!("{:#?}", e.0);
    }
}

pub async fn async_relay<'a, 'b>(reader: TopicReader<'a, A>, writer: TopicWriter<'b, B>) {
    writer.write_all(
        read_all!(reader)
            .iter()
            .map(|x| B(x.0.to_owned() + " world")),
    );
}

fn try_fn() -> AnyResult<()> {
    bail!("Sync pipe can return an error")
}

async fn async_try_fn() -> AnyResult<u8> {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    bail!("Async pipe can return an error")
}

fn main() {
    let mut app = Zmaxion::default();
    let mut builder = app.builder();
    builder
        .add_bevy_plugin(LogPlugin)
        .add_plugins(DefaultPlugins)
        .add_plugin(LogErrorsPlugin);
    builder
        .world
        .get_resource::<LogErrorsSynchronously>()
        .unwrap()
        .0
        .store(true, Ordering::SeqCst);

    builder
        .register_topic::<A>()
        .register_topic::<B>()
        .register_pipe(transmitter, None)
        .register_pipe(relay, None)
        .register_pipe(try_fn, None)
        .register_pipe(async_transmitter, None)
        .register_pipe(async_relay, None)
        .register_pipe(async_try_fn, None);
    drop(builder);
    // this structure can be serialized/deserialized
    app.spawn_pipeline(SpawnPipeline(Arc::new(SpawnPipelineInner {
        id: 0,
        name: "test".to_string(),
        topics: vec![
            Arc::new(TopicConfig {
                name: "A".into(),
                connector: "".to_string(),
                schema: A::type_name().into(),
                initial_message_set: vec![],
                n_initial_message_sets: 0,
                args: vec![],
                lifetime: TopicLifetime::Global,
                access: TopicAccess::Private,
                idempotence: Idempotence::No,
                async_support: AsyncSupport::No,
                transactional: TransactionSupport::No,
            }),
            Arc::new(TopicConfig {
                name: "B".into(),
                connector: "".to_string(),
                schema: B::type_name().into(),
                initial_message_set: vec![],
                n_initial_message_sets: 0,
                args: vec![],
                lifetime: TopicLifetime::Global,
                access: TopicAccess::Private,
                idempotence: Idempotence::No,
                async_support: AsyncSupport::No,
                transactional: TransactionSupport::No,
            }),
            Arc::new(TopicConfig {
                name: "State".into(),
                connector: "kafka".to_string(),
                schema: PipelineState::type_name().into(),
                initial_message_set: vec![],
                n_initial_message_sets: 0,
                args: vec![],
                lifetime: TopicLifetime::Pipeline(".".into()),
                access: TopicAccess::Private,
                idempotence: Idempotence::No,
                async_support: AsyncSupport::No,
                transactional: TransactionSupport::No,
            }),
        ],
        pipes: vec![
            Arc::new(PipeConfig {
                name: type_name_of!(try_fn).into(),
                args: vec![],
                reader_topics: smallvec![],
                writer_topics: smallvec![],
            }),
            Arc::new(PipeConfig {
                name: type_name_of!(async_try_fn).into(),
                args: vec![],
                reader_topics: smallvec![],
                writer_topics: smallvec![],
            }),
            Arc::new(PipeConfig {
                name: type_name_of!(async_transmitter).into(),
                args: vec![],
                reader_topics: smallvec!["B".into()],
                writer_topics: smallvec!["A".into()],
            }),
            Arc::new(PipeConfig {
                name: type_name_of!(async_relay).into(),
                args: vec![],
                reader_topics: smallvec!["A".into()],
                writer_topics: smallvec!["B".into()],
            }),
            Arc::new(PipeConfig {
                name: type_name_of!(transmitter).to_string(),
                args: vec![],
                reader_topics: smallvec!["B".into()],
                writer_topics: smallvec!["A".into()],
            }),
            Arc::new(PipeConfig {
                name: type_name_of!(relay).to_string(),
                args: vec![],
                reader_topics: smallvec!["B".into()],
                writer_topics: smallvec!["B".into()],
            }),
        ],
        args: Default::default(),
        state_generation: 0,
        state_connector: "kafka".to_string(),
    })));

    app.run();
}
