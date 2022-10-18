#![feature(trace_macros)]
extern crate core;

use std::sync::atomic::{AtomicPtr, AtomicUsize};
pub use bevy_ecs;
pub use smallvec;
use zmaxion_rt::tokio;

pub mod components;
pub mod error;
pub mod macros;
pub mod messages;
pub mod models;
// pub mod v2;
// pub mod pipe;
pub mod ext;
pub mod resources;
pub mod traits;

// pub mod topic;

pub mod prelude {
    pub use bevy_derive::{Deref, DerefMut};
    pub use bevy_ecs::{self, prelude::*, system::Resource};
    pub use bevy_utils::{HashMap, HashSet};
    pub use derive_new::new;
    pub use ext::EntityExt;
    pub use smallvec::{smallvec, SmallVec};

    use crate::ext;
    // If schedule is rebuilt then these trackers could trigger systems thus making them
    // private
    //    #[allow(unused_imports)]
    //    use bevy::prelude::{Added, Changed, RemovedComponents};
    //    #[allow(unused_imports)]
    //    use bevy::prelude::{DefaultPlugins, Plugin, PluginGroup};
    //    pub use bevy::{
    //        ecs as bevy_ecs,
    //        prelude::{Plugin as BevyPlugin, PluginGroup as BevyPluginGroup, *},
    //        utils::{HashMap, HashSet},
    //    };
    pub use crate::{
        components::Name,
        messages::SpawnPipeline,
        models::{config::*, SpawnPipelineInner},
        read, read_all, read_all_loop, read_loop,
        resources::GlobalEntity,
        traits::WorldExt,
    };
}

pub mod markers {
    use paste::paste;
    use zmaxion_derive::{all_supertraits, all_supertraits_for};

    pub trait TopicReader<'s, T> {}
    pub trait TopicWriter<'s, T> {}
    pub trait Ack {}
    pub trait Async {}
    pub trait Bevy {}
    pub trait Group {}
    pub trait LowLatency {}
    pub trait Realtime {}
    pub trait Trace {}

    macro_rules! permute {
        ([$main:ident $(< $($life:lifetime,)*  $($generics:ident),+ >)?]; $first:ident, $($others:ident),+) => {
            paste!{
                pub trait [<$main $first $($others)+>]$(<$($life,)* $($generics),+>)?: $main $(<$($life,)* $($generics),+>)? + $first + $($others+)+ {}
                impl<
                    $($($life,)+)?
                    __T: $first + $($others+) + $main <$($($life,)* $($generics),+>)?
                    $($(,$generics)+)?
                > [<$main $first $($others)+>]$(<$($life,)* $($generics),+>)? for __T {
                }
            }
            permute!([$main $(<$($life,)* $($generics),+>)?]; $($others),+);
        };
        ([$main:ident $(<$($life:lifetime,)* $($generics:ident),+>)*]; $first:ident $(,)?) => {
        };
    }

    all_supertraits_for!(['s] [T] TopicReader<'s, T>, Async, Ack, Group, Realtime, Trace);
    all_supertraits_for!(['s] [T] TopicReader<'s, T>, Bevy, Ack, Group, Realtime, Trace);

    //    all_supertraits_for!(['s] [T] TopicReader<'s, T>, Async, Ack, Group, LowLatency, Trace);
    //    all_supertraits_for!(['s] [T] TopicReader<'s, T>, Bevy, Ack, Group, LowLatency, Trace);
}

// pub mod flow;
// pub mod flower;

// struct A<'a>(PhantomData<&'a ()>);

fn a() {
    // app.add_plugin(|| ().plugin_label(PluginLabel).deps([PluginA]));
    // app.add_pipe(pipe.declaration(PipeDeclaration {
    //      idempotency: x,
    //      ..Default::default()
    // });
    // app.add_generic_pipe::<(T, U)>(|builder| builder.add(generic_pipe::<T, U>).before(label)
    // .validator(my_validator))
    //    test(|| ());
}

mod schedule {
    /*
    4 kinds of systems:
    free form system - has no dependency, get executed whenever executor has nothing else to do
    listener - gets scheduled when some other system has invalidated what this listener is accessing
    dependant - needs to wait for a dependency to run
    one off systems - gets read from a channel and executed once
    init systems - systems that initialize other systems
    destructor systems - systems that run once system has despawned

    entities and components with different storage backend

    access_kind: read, write, rw, interior_write

    commands:
    systems could depend on run_commands system that has exclusive access to schedule this will wait
    for all systems to complete then run system and then check if there are any commands
    todo: figure out a way if we could split commands into entity commands and schedule so that we
    dont wait for systems to finish

    portals - async system can fetch ecs data while it is running but it didn't have that
    access defined, await for scheduler to make room for acces then return to future

    subworlds - have an array of an array so that world_id could get passed, duplicate jobs and
    access

    async runtimes: prioritize spawning on thread local runtime, users can specify to spawn on
    global runtime, either a job or a future

    plugins: load plugin at runtime and unload it, specify its dependencies

    mpmc atomic queues: can be accesses mutably for less overhead if paralelism isnt needed
    mpmr qtomic topics
    todo: figure out a way if we could send an id but the receving end is reading component instead

    workflows: is a DAG, once a node is set to running state it spawns all associated systems,
    once some system sends an event that workflow is done then advnce to next nodes, if failed then
    advance to next nodes associated for failure

    parallel systems:
     */
    use std::{
        collections::VecDeque,
        marker::PhantomData,
        sync::{
            atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
            mpsc, Arc,
        },
    };
    use std::any::Any;
    use std::future::Future;
    use std::ops::Range;
    use std::pin::Pin;
    use std::ptr::NonNull;
    use std::sync::atomic::AtomicIsize;
    use std::task::{Context, Poll, ready};
    use std::time::Duration;

    use async_watch::Ref;
    use atomic::Atomic;
    use bevy_utils::BoxedFuture;
    use concurrent_queue::ConcurrentQueue;
    use ergnomics::prelude::*;
    use fixedbitset::FixedBitSet;
    use futures::{channel::mpsc::{Receiver, Sender}, future::FutureExt, pin_mut};
    use futures_util::{future::BoxFuture, stream::FuturesUnordered, SinkExt, StreamExt};
    use futures_util::future::Either;
    use futures_util::stream::Next;
    use nested::Nested;
    use parking_lot::{Condvar, Mutex, RawMutex, RwLock};
    use parking_lot::lock_api::MutexGuard;
    use petgraph::{graph::EdgeIndex, Graph};
    use zmaxion_rt::{JoinHandle, Notify, Runtime, tokio};
    use pin_project_lite::pin_project;

    struct WorldAccessState {
        current_access: Mutex<CurrentAccess>
    }

    struct SystemContainer {
        world: &'static WorldAccessState,
        access: Vec<AccessKind>,
        system: Box<dyn ExternalJob>,
    }

    trait Trait {}

    impl Trait for () {
    }

    fn test<F, T>(f: F)
    where
        F: FnOnce() -> T,
        T: Trait,
    {
    }

    pub trait Pipe {
        fn run_async(&mut self) -> BoxFuture<()> {
            async {}.boxed()
        }
        fn run(&mut self) {
        }
    }
    pub struct AsyncPipe {}
    enum AccessKind {
        None,
        ReadAll,
        ReadSpecific(Vec<usize>),
        InteriorWriteAll,
        InteriorWriteSpecific(Vec<usize>),
        WriteAll,
        Writepecific(Vec<usize>),
    }

    // 3 kinds of jobs:
    // free form jobs: get run whenever there are no other jobs to run
    // listeners: get run whenever a param they access has changed, cannot have dependencies
    // dependants: get run when all dependencies have changed

    // things that futures do but this doesn't
    // no check if a mutex is locked for each message of a channel
    // no check if waker is present for each message of a channel
    // no dynamic dispatch to wake other futures
    //
    struct JobPipeline {
        handles: Vec<std::thread::JoinHandle<()>>,
        pipeline: Arc<SharedJobPipeline>,
    }

    struct CurrentAccess {
        all_mut: FixedBitSet,
        all_ref: FixedBitSet,
        specific_mut: Vec<Vec<Range<usize>>>,
        specific_ref: Vec<Vec<Range<usize>>>,
    }
    struct SharedListeners {
        // jobs which paramaters have changed
        // if a job that has a paramater with interior mutability or exterior then when it has
        // finished it goes through all jobs that have access to changed paramaters and schedules
        // them for execution. Also contains jobs which are waiting for
        // dependencies/pipelined dependants.
        scheduled_listeners_for_execution: VecDeque<ListenerId>,
    }
    struct SharedJobPipelineState {
        // free form pipes - either take no access or are declared as free form
        // they get executed first or when there are no jobs left to execute
        // If we have 2 threads in pool with 3 total free form jobs in schedule. Job 0 reads A.
        // Job 1 reads B. Job 2 writes to A and B. If there are no generations, then only job 0 and
        // 1 would be run. 2 cannot run because of access. If ambigous order is detected.
        // To slove this, don't make them all free form.
        // - a - b - c - a - b - d
    }

//    struct RootJobs {
//        needed_access: Nested<Vec<NeededAccess>>,
//        // jobs that need to run after this one
//        next_jobs: Nested<Vec<JobId>>,
//    }
//
//    struct Listeners {
//        needed_access: Nested<Vec<NeededAccess>>,
//        // jobs that need to run after this one
//        next_jobs: Nested<Vec<JobId>>,
//    }

    // Spawn free form jobs
    // once finished
    // decrement position of each dependant if 0 spawn
    // in a loop:
    // once a job has finished:
    // for each dependant:
    // decrement position
    // if position 0 spawn
    // else put in scheduled for execution
    // NOTE: linked jobs must have ambigous access
    struct Jobs {
        needed_access: Nested<Vec<NeededAccess>>,

        // max possible jobs 2^63
        // it is a sum of:
        // current_n_dependencies: AtomicUsize,
        // current_n_dependendants: AtomicUsize,
        // current_n_dependency_state_machines: AtomicUsize,
        // current_n_dependant_state_machines: AtomicUsize,
        // when position reaches 0 it is ready to wait for access
        position: Vec<AtomicUsize>,
        // jobs that need to run after this one
        next_jobs: Nested<Vec<JobId>>,

//        next_jobs: NonNull<JobId>,
        // we implicitly know that capacity is next power of 2
//        next_jobs_len: usize,
        // Every dependant of a dependant that has ambigous access with this one.
        // Every job that has ambigous access with this one needs to have finished. For example:
        // clear and add job must not happen if read hasn't happened in previous generation.
        // clear and add -> modify -> read
        // Those aren't strictly dependants because it could be dependants of dependants.
        // jobs that dont have ambigous access but are dependant could run more than once in a graph
        recurisve_prev_jobs_with_ambigous_access: Nested<Vec<JobId>>,
    }

    enum ExternalJobOutput {
        RunNext {
            output: Box<dyn Any>,
            id: Box<dyn ExternalJob>,
        },
        Spawned,
    }

    enum JobOutput<T> {
        RunNext {
            output: T,
            id: JobId,
        },
        Spawned,
    }

    trait Job {
        type Output;
        fn run(&mut self, runtime: &Runtime) -> JobOutput<Self::Output>;
    }

    trait ExternalJob {
        fn run(&mut self, runtime: &Runtime) -> ExternalJobOutput;
    }

    struct SharedJobPipeline {
        // TODO: specialize on function pointers, figure out how to construct stateless paramaters
        //  (or maybe even stateful) and pass them to function
        // sub worlds, systems can query entities from different worlds (at runtime) without any
        // locks
        // parallel systems, systems that can be split in multiple sub systems
        // portal - async system can fetch ecs data while it is running but it didn't have that
        // access defined, await for scheduler to make room for acces then return to future
        runtime: Runtime,
        listeners_per_access: Nested<Vec<ListenerId>>,
        jobs_waiting_for_access: ConcurrentQueue<JobId>,
        external_jobs_waiting_for_access: ConcurrentQueue<Arc<dyn ExternalJob>>,

        scheduled_listeners_for_execution_set: Mutex<FixedBitSet>,
        waiting_for_access: ConcurrentQueue<JobId>,
        current_access: Mutex<CurrentAccess>,

        free_job_slots: Vec<usize>,
        // they must start once there is no access from jobs that are scheduled for execution and
        // jobs that are running
        free_form_jobs: Vec<JobId>,
        jobs: Jobs,
        state: Mutex<SharedJobPipelineState>,
        // max enqueued items per worker = num_cpus
        enqueued_jobs: Vec<Mutex<VecDeque<JobId>>>,
        finished_jobs: Mutex<VecDeque<JobId>>,
        stage: Mutex<PipelineStage>,
        state_condvar: Condvar,
        paused_workers: AtomicUsize,
        pipeline_starved: Mutex<()>,
    }

    impl SharedJobPipeline {
        fn n_workers(&self) -> usize {
            self.enqueued_jobs.len()
        }
    }

    #[derive(PartialEq, Copy, Clone)]
    enum PipelineStage {
        BeginPause,
        Paused,
        Running,
        RunningFuturesOnly,
        Exit,
    }

    impl JobPipeline {
        pub fn new(n_workers: usize) -> Self {
            tokio::spawn()
            let pipeline = Arc::new(SharedJobPipeline {
                max_generation: Default::default(),
                runtime: Default::default(),
                free_job_slots: vec![],
                jobs: vec![],
                state: Default::default(),
                enqueued_jobs: (0..n_workers).map(|_| ConcurrentQueue::unbounded()).collect(),
                finished_jobs: Default::default(),
                stage: Mutex::new(PipelineStage::Paused),
                state_condvar: Default::default(),
                pipeline_starved: Default::default(),
            });
            let handles: Vec<_> = (0..n_workers)
                .map(|x| {
                    let pipeline = pipeline.clone();
                })
                .collect();
            JobPipeline {
                handles,
                pipeline,
            }
        }
    }

    fn enq_jobs(pipeline: &SharedJobPipeline, current_access: &mut SharedJobPipelineState) {
        for job_id in &current_access.waiting_for_access {
            let job = pipeline.jobs[*job_id as usize];
            for needed_access in &job.needed_access {
                match current_access[needed_access.id as usize] {
                    AccessKind::ReadAll => {}
                    AccessKind::ReadSpecific(_) => {}
                    AccessKind::WriteAll => {}
                    AccessKind::Writepecific(_) => {}
                    AccessKind::None {}
                }
            }
        }
    }

    struct Worker<'a> {
        id: usize,
        pipeline: Arc<SharedJobPipeline>,
        futures: FuturesUnordered<JobFuture>,
        future: Pin<&'a mut Option<Next<'a, FuturesUnordered<JobFuture>>>>,
    }
    pin_project! {
        struct JobFuture {
            #[pin]
            handle: JoinHandle<()>,
            job_id: JobId,
        }
    }

    impl Future for JobFuture {
        type Output = JobId;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            ready!(this.handle.poll(cx));
            Poll::Ready(self.job_id)
        }
    }

    impl<'a> Worker<'a> {
        pub fn spawn(pipeline: Arc<SharedJobPipeline>, id: usize) -> std::thread::JoinHandle<()> {
            std::thread::spawn(move || {
                let future;
                pin_mut!(future);
                Worker {
                    id,
                    pipeline,
                    futures: Default::default(),
                    future,
                }.run()
            })
        }

        fn run(&mut self) {
            loop {
                loop {
                    let queue = self.pipeline.enqueued_jobs[self.id].lock();
                    match queue.pop_front() {
                        Some(job_id) => {
                            self.run_job(job_id);
                        }
                        None => {
                            break;
                        }
                    }
                }
                self.any_future_finished();
                let mut guard = self.pipeline.stage.lock();
                let state = guard.clone();
                drop(guard);
                match state {
                    PipelineStage::BeginPause => {
                        self.pause();
                    }
                    PipelineStage::Paused => {
                        let mut state = self.pipeline.stage.lock();
                        self.pipeline.state_condvar.wait_while(
                            &mut state,
                            |x| *x == PipelineStage::Paused
                        );
                    }
                    PipelineStage::Running => {}
                    PipelineStage::Exit => {
                        break;
                    }
                    PipelineStage::RunningFuturesOnly => unimplemented!(),
                }
                if let Some(mut current_access) = self.pipeline.state.try_lock() {
                    self.enq_jobs(&mut *current_access);
                    drop(current_access);
                    if self.pipeline.enqueued_jobs[self.id].is_empty() {
                        if !self.try_steal_and_run() {
                            self.wait();
                        }
                    }
                } else {
                    if !self.try_steal_and_run() {
                        self.wait();
                    }
                }
            }
        }

        fn pause(&mut self) {
            loop {
                if !self.try_steal_and_run() {
                    break;
                }
            }
            match self.future.as_pin_mut() {
                None => {}
                Some(fut) => {
                    if let Some(id) = self.pipeline.runtime.block_on(fut) {
                        self.job_finished(id);
                    }
                }
            }
            while let Some(id) = self.pipeline.runtime.block_on(self.futures.next()) {
                self.job_finished(id);

            }
            if self.pipeline.paused_workers.fetch_add(1, Ordering::Relaxed)  == self.pipeline.n_workers() - 1{
                self.pipeline.paused_workers.fetch_and(0, Ordering::Relaxed);
                let mut state = self.pipeline.stage.lock();
                *state = PipelineStage::Paused;
                drop(state);
                self.pipeline.state_condvar.notify_all();
                let mut state = self.pipeline.stage.lock();
                self.pipeline.state_condvar.wait_while(
                    &mut state,
                    |x| *x == PipelineStage::Paused
                );
            }
        }

        fn any_future_finished(&mut self) -> bool {
            match self.future.as_pin_mut() {
                None => {
                    *self.future = Some(self.futures.next());
                    match self.future.as_pin_mut().unwrap().now_or_never().flatten() {
                        None => false,
                        Some(id) => {
                            self.job_finished(id);
                            true
                        }
                    }
                }
                Some(fut) => {
                    match fut.now_or_never().flatten() {
                        None => false,
                        Some(id) => {
                            self.job_finished(id);
                            true
                        }
                    }
                }
            }
        }

        fn job_finished(&mut self, job_id: JobId) {

        }

        fn wait(&mut self) {
            let mut stage = self.pipeline.stage.lock();

            let mut millis = 2;
            loop {
                if self.any_future_finished() {
                    break;
                }
                if let Some(guard) = self.pipeline.stage.try_lock() {
                    if *guard == PipelineStage::BeginPause {
                        break;
                    }
                }
                let sleep = zmaxion_rt::sleep(Duration::from_millis(millis));
                let next = self.futures.next();
                pin_mut!(sleep);
                pin_mut!(next);
                match self.pipeline.runtime.block_on(
                    futures::future::select(
                        sleep,
                        next
                    )
                ) {
                    Either::Left(_) => {
                        millis = millis * millis;
                    }
                    Either::Right(_) => {
                        break
                    }
                }
            }
        }

        fn run_job(&mut self, job_id: JobId) {

        }

        fn enq_jobs(&mut self, state: &mut SharedJobPipelineState) {

        }

        fn try_steal_and_run(&mut self) -> bool {
            let start = rand::random::<usize>() % self.pipeline.n_workers();
            for i in start..start + self.pipeline.n_workers() {
                if i == self.id {
                    continue;
                }
                if let Ok(job_id) = self.pipeline.enqueued_jobs[i].pop() {
                    self.run_job(job_id);
                    return true;
                }
            }
            false
        }

    }
    // free jobs

    type JobId = usize;
    type ListenerId = JobId;
    type StateId = JobId;
    type AccessId = usize;
    type EdgeId = JobId;
    type JobEdgeId = EdgeIndex<JobId>;

    struct NeededAccess {
        id: AccessId,
        kind: AccessKind,
    }
}

mod v2 {
    use petgraph::{graph::EdgeIndex, prelude::GraphMap, Directed, Graph};
    use legion::Schedule;

    type JobId = usize;

    struct Jobs {
        graph: Graph<JobId, (), usize>,
        a: GraphMap<(), (), Directed>,
    }
}

// #![feature(trace_macros)]
// extern crate core;
//
// pub use bevy_ecs;
// pub use smallvec;
//
// pub mod components;
// pub mod error;
// pub mod macros;
// pub mod messages;
// pub mod models;
// pub mod v2;
// pub mod pipe;
// pub mod ext;
// pub mod resources;
// pub mod traits;
//
// pub mod topic;
//
// pub mod prelude {
// pub use bevy_derive::{Deref, DerefMut};
// pub use bevy_ecs::{self, prelude::*, system::Resource};
// pub use bevy_utils::{HashMap, HashSet};
// pub use derive_new::new;
// pub use ext::EntityExt;
// pub use smallvec::{smallvec, SmallVec};
//
// use crate::ext;
// If schedule is rebuilt then these trackers could trigger systems thus making them
// private
//    #[allow(unused_imports)]
//    use bevy::prelude::{Added, Changed, RemovedComponents};
//    #[allow(unused_imports)]
//    use bevy::prelude::{DefaultPlugins, Plugin, PluginGroup};
//    pub use bevy::{
//        ecs as bevy_ecs,
//        prelude::{Plugin as BevyPlugin, PluginGroup as BevyPluginGroup, *},
//        utils::{HashMap, HashSet},
//    };
// pub use crate::{
// components::Name,
// messages::SpawnPipeline,
// models::{config::*, SpawnPipelineInner},
// read, read_all, read_all_loop, read_loop,
// resources::GlobalEntity,
// traits::WorldExt,
// };
// }
//
// pub mod markers {
// use paste::paste;
// use zmaxion_derive::{all_supertraits, all_supertraits_for};
//
// pub trait TopicReader<'s, T> {}
// pub trait TopicWriter<'s, T> {}
// pub trait Ack {}
// pub trait Async {}
// pub trait Bevy {}
// pub trait Group {}
// pub trait LowLatency {}
// pub trait Realtime {}
// pub trait Trace {}
//
// macro_rules! permute {
// ([$main:ident $(< $($life:lifetime,)*  $($generics:ident),+ >)?]; $first:ident,
// $($others:ident),+) => { paste!{
// pub trait [<$main $first $($others)+>]$(<$($life,)* $($generics),+>)?: $main $(<$($life,)*
// $($generics),+>)? + $first + $($others+)+ {} impl<
// $($($life,)+)?
// __T: $first + $($others+) + $main <$($($life,)* $($generics),+>)?
// $($(,$generics)+)?
// > [<$main $first $($others)+>]$(<$($life,)* $($generics),+>)? for __T {
// }
// }
// permute!([$main $(<$($life,)* $($generics),+>)?]; $($others),+);
// };
// ([$main:ident $(<$($life:lifetime,)* $($generics:ident),+>)*]; $first:ident $(,)?) => {
// };
// }
//
// all_supertraits_for!(['s] [T] TopicReader<'s, T>, Async, Ack, Group, Realtime, Trace);
// all_supertraits_for!(['s] [T] TopicReader<'s, T>, Bevy, Ack, Group, Realtime, Trace);
//
//    all_supertraits_for!(['s] [T] TopicReader<'s, T>, Async, Ack, Group, LowLatency, Trace);
//    all_supertraits_for!(['s] [T] TopicReader<'s, T>, Bevy, Ack, Group, LowLatency, Trace);
// }
//
// pub mod flow;
// pub mod flower;
//
// struct A<'a>(PhantomData<&'a ()>);
//
// fn a() {
// app.add_plugin(|| ().plugin_label(PluginLabel).deps([PluginA]));
// app.add_pipe(pipe.declaration(PipeDeclaration {
//      idempotency: x,
//      ..Default::default()
// });
// app.add_generic_pipe::<(T, U)>(|builder| builder.add(generic_pipe::<T, U>).before(label)
// .validator(my_validator))
//    test(|| ());
// }
//
// mod schedule {
// use std::{
// marker::PhantomData,
// sync::{
// atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
// mpsc, Arc,
// },
// };
//
// use ergnomics::prelude::*;
// use futures::{
// channel::mpsc::{Receiver, Sender},
// future::FutureExt,
// };
// use futures_util::{future::BoxFuture, stream::FuturesUnordered, SinkExt, StreamExt};
// use parking_lot::{Mutex, RwLock};
// use petgraph::Graph;
// use petgraph::graph::EdgeIndex;
// use zmaxion_rt::{JoinHandle, Notify, Runtime};
//
// trait Trait {}
//
// impl Trait for () {
// }
//
// fn test<F, T>(f: F)
// where
// F: FnOnce() -> T,
// T: Trait,
// {
// }
//
// pub trait Pipe {
// fn run_async(&mut self) -> BoxFuture<()> {
// async {}.boxed()
// }
// fn run(&mut self) {
// }
// }
// pub struct AsyncPipe {}
// enum AccessKind {
// All,
// Specific(usize),
// }
//
// struct JobPipeline {
// min_generation: AtomicUsize,
// max_generation: AtomicUsize,
// population_scratch: Mutex<Vec<JobPopulationContainer>>,
// pipeline: RwLock<Vec<JobPopulationContainer>>,
// enque_jobs: async_channel::Sender<JobId>,
// enqueued_jobs: async_channel::Receiver<JobId>,
// }
// free jobs
//
// type JobId = usize;
// type StateId = JobId;
// type AccessId = usize;
// type EdgeId = JobId;
// type JobEdgeId = EdgeIndex<JobId>;
//
// struct JobPopulationContainer {
// If we have 2 threads in pool with 3 total jobs in schedule. Job 0 reads A. Job 1 reads
// B. Job 2 writes to A and B. If there are no generations, then only job 0 and 1
// would be run. 2 cannot run because of access. If ambigous order is detected
// generation: u64,
// population: Mutex<JobPopulation>,
// }
//
// struct JobPopulation {
// Every job that has ambigous access with this one needs to have finished. For example:
// clear and add job must not happen if read hasn't happened in previous generation.
// clear and add -> modify -> read
// Those aren't strictly dependants because it could be dependants of dependants.
// n_active_state_machines: JobId,
// waiting_for_state_machines: Vec<ActiveStateMachinesForJob>,
// waiting_for_dependencies: Vec<JobId>,
// waiting_for_access: Vec<JobId>,
// }
//
// struct ActiveStateMachinesForJob {
// job_id: JobId,
// active_state_machines: Vec<JobId>,
// }
//
// struct Jobs {
// If we have 2 threads in pool with 3 total jobs in schedule. Job 0 reads A. Job 1 reads
// B. Job 2 writes to A and B. If there are no generations, then only job 0 and 1
// would be run. 2 cannot run because of access. If ambigous order is detected
// generation: Vec<AtomicU64>,
// suspended: Vec<AtomicBool>,
// There must not be any enqueued writes for specific access.
// read_access: Vec<Vec<AccessId>>,
// There must not be any enqueued writes or reads for specific access.
// write_access: Vec<Vec<AccessId>>,
// Jobs that could resolve to race condition if run in parallel.
// ambigous_jobs: Vec<Vec<JobId>>,
// dependencies: Vec<Vec<JobId>>,
// dependants: Vec<Vec<JobId>>,
// Every dependant of a dependant that has ambigous access with this one.
// state_machines: Vec<Vec<JobId>>,
// if generation of all dependencies is bigger by 1 from ours then proceed
// if generation of all dependants is equal to ours then proceed
// if max generation difference of all ambigous jobs is bigger than threshold then skip
// if access is ok then run
// }
//
// struct Edge {
// from: JobId,
// to: JobId,
// }
//
// struct Job {
//
// }
//
// struct Graphs {
// dependency_graph: Graph<JobId, (), JobId>,
// ambigous_jobs_graph:Graph<JobId, (), JobId>,
// state_machine_graph: Graph<JobId, (), JobId>,
// }
//
// enum Execution {
// Async(Vec<usize>),
// Sync(usize),
// Notify(Sender<()>),
// Wait(Receiver<()>),
// }
//
// pub struct StagedSchedule {
// runtime: Runtime,
// context: Arc<Context>,
// }
//
// pub struct Context {
// pipes: Vec<Box<dyn Pipe>>,
// execution_plan: Vec<Execution>,
// min_running_pipe_id: AtomicUsize,
// n_workers: usize,
// n_finished_workers: AtomicUsize,
// stage_finished_sender: Sender<()>,
// }
//
// enum ExecutorCommand {
// Start,
// Exit,
// }
//
// struct ParallelExecutor {
// start_work: async_channel::Sender<ExecutorCommand>,
// end_work: async_channel::Receiver<ExecutorCommand>,
// }
//
// impl ParallelExecutor {
// pub fn new(n_workers: usize) -> Self {
// let context: Context = Arc::new(Context);
// let start_work = Arc::new(Notify::new());
// let end_work = Arc::new(Notify::new());
// let (start_sender, start_recv) = mpsc::channel();
// let (end_sender, end_recv) = mpsc::channel();
// for i in 0..n_workers {
// let start_work = start_work.clone();
// let end_work = end_work.clone();
// std::thread::spawn(move || {
// let runtime = Runtime::default();
// loop {
// runtime.block_on(start_work.notified());
// loop {
// let id = context.min_running_pipe_id.fetch_add(1, Ordering::Relaxed);
// if id >= context.execution_plan.len() {
// break;
// }
// match &context.execution_plan[id] {
// Execution::Async(ids) => unsafe {
// let mut futs: FuturesUnordered<_> = ids
// .iter()
// .map(|x| {
// context
// .pipes
// .get_unchecked(id)
// .as_mut_cast()
// .run_async()
// })
// .collect();
// while let Some(_) = runtime.block_on(futs.next()) {}
// },
// Execution::Sync(id) => unsafe {
// context.pipes.get_unchecked(*id).as_mut_cast().run();
// },
// Execution::Notify(sender) => unsafe {
// sender.as_mut_cast().start_send(());
// },
// Execution::Wait(recv) => unsafe {
// runtime.block_on(recv.as_mut_cast().next());
// },
// }
// }
// end_work.notify_one();
// }
// });
// }
// Self {}
// }
//
// fn run_once(&mut self) {
// }
// }
//
// impl StagedSchedule {
// }
// }
//
// mod v2 {
// use petgraph::Graph;
// use petgraph::graph::EdgeIndex;
//
// type JobId = usize;
//
// struct Jobs {
// graph: Graph<JobId, (), usize>,
// a: EdgeIndex,
// }
// }
