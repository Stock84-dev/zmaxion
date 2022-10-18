use std::sync::{atomic::Ordering, Arc};

use zmaxion::{prelude::*, rt::tokio};
use zmaxion_internal::{
    core::{components::PipelineState, resources::LogErrorsSynchronously},
    param::IntoParamStateMut,
    plugins::LogErrorsPlugin,
};

pub struct A(String);
pub struct B(String);

fn relay(reader: AsyncReader<B>, mut writer: AsyncWriter<B>) {
    writer.extend(
        reader
            .try_read()
            .iter()
            .map(|x| x.iter())
            .flatten()
            .map(|x| B(x.0.to_owned() + " world")),
    );
}

fn transmitter(mut writer: AsyncWriter<A>, reader: AsyncReader<B>) {
    writer.extend_one(A("hello".into()));
    for e in read_all!(reader) {
        debug!("{:#?}", e.0);
    }
}

pub async fn async_transmitter(
    mut writer: AsyncWriter<'static, A>,
    reader: AsyncReader<'static, B>,
) -> AnyResult<()> {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    writer.extend_one(A("hello".into()));
    for e in reader.read().await?.iter() {
        debug!("{:#?}", e.0);
    }
    Ok(())
}

pub async fn async_relay<'a, 'b>(
    reader: AsyncReader<'a, A>,
    writer: AsyncWriter<'b, B>,
) -> AnyResult<()> {
    writer.extend(
        reader
            .read()
            .await?
            .iter()
            .map(|x| B(x.0.to_owned() + " world")),
    );
    Ok(())
}

fn try_fn() -> AnyResult<()> {
    //    bail!("Sync pipe can return an error")
    Ok(())
}

async fn async_try_fn() -> AnyResult<u8> {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    trace!("where");
    //    bail!("Async pipe can return an error")
    Ok(0)
}

fn main() {
    let mut app = Zmaxion::default();
    let mut builder = app.builder();
    builder
        .add_plugins(DefaultPlugins)
        .add_plugin(LogErrorsPlugin);
    builder
        .world
        .get_resource::<LogErrorsSynchronously>()
        .unwrap()
        .0
        .store(true, Ordering::Relaxed);

    builder
        .register_topic::<A>()
        .register_topic::<B>()
        .register_pipe(transmitter)
        .register_pipe(relay)
        .register_pipe_with(try_fn, serial_pipe_features())
        .register_pipe(async_transmitter)
        .register_pipe(async_relay)
        .register_pipe(async_try_fn);
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
            //            Arc::new(TopicConfig {
            //                name: "State".into(),
            //                connector: "kafka".to_string(),
            //                schema: PipelineState::type_name().into(),
            //                initial_message_set: vec![],
            //                n_initial_message_sets: 0,
            //                args: vec![],
            //                lifetime: TopicLifetime::Pipeline(".".into()),
            //                access: TopicAccess::Private,
            //                idempotence: Idempotence::No,
            //                async_support: AsyncSupport::No,
            //                transactional: TransactionSupport::No,
            //            }),
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
