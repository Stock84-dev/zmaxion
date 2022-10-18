use std::{
    collections::{LinkedList, VecDeque},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{
            AtomicU32, AtomicU64, AtomicUsize, Ordering,
            Ordering::{Relaxed, SeqCst},
        },
        mpsc, Arc,
    },
    task::{ready, Context, Poll},
};

use atomic::Atomic;
use ergnomics::prelude::*;
use fixedbitset::FixedBitSet;
use futures::{future::Then, stream::FuturesUnordered, FutureExt, StreamExt};
use vec_list::VecList;
use zmaxion_rt::{prelude::Handle, JoinHandle, Notify, Runtime};
use zmaxion_utils::prelude::{error, Mutex};

mod commands;
pub mod system;

use crate::{
    archetype::{ArchetypeComponentId, ArchetypeId},
    data_types::SparseVec,
    entity::Entity,
    parallel_executor::system::SystemId,
    storage::SparseArray,
    world::World,
};

pub struct ParallelExecutor {}

impl ParallelExecutor {
    pub fn new() -> Self {
        Self::with_worker_count(num_cpus::get())
    }

    pub fn with_worker_count(count: usize) -> Self {
        let shared = Arc::new(WorkerShared {
            time_to_execute: 0,
            enqueued: vec![],
            start_running: Default::default(),
            systems: Systems {},
            access: vec![],
        });
        let handles = (0..count)
            .map(|worker_id| {
                let shared = shared.clone();
                std::thread::spawn(move || {
                    let rt = Runtime::default_current_thread();
                    rt.block_on(async {
                        let worker = Worker::new(worker_id, shared).await;
                        worker.run()
                    })
                })
            })
            .into_vec();
        Self {}
    }
}

impl ParallelExecutor {
    pub fn run(&mut self, worlds: &mut SparseVec<World>) {
    }
}

struct Worker {
    id: usize,
    shared: Arc<WorkerShared>,
    async_systems: FuturesUnordered<SystemFuture>,
    finished_stack: Vec<SystemId>,
    handle: Handle,
}

impl Worker {
    async fn new(worker_id: usize, shared: Arc<WorkerShared>) -> Self {
        Self {
            id: worker_id,
            shared,
            async_systems: Default::default(),
            finished_stack: vec![],
            handle: Handle::current(),
        }
    }

    async fn run(&mut self) {
        loop {
            loop {
                let mut queue = self.shared.enqueued[self.id].lock();
                let id = match queue.pop_front() {
                    Some(id) => id,
                    None => {
                        break;
                    }
                };
                drop(queue);
                self.run_system(id, None);
            }
            self.poll_futures(None).await;
            match self.shared.state.load(SeqCst) {
                ExecutorState::Pausing => {
                    self.pause().await;
                    match self.shared.state.load(SeqCst) {
                        ExecutorState::Pausing => {}
                        ExecutorState::Running => {}
                        ExecutorState::Exit => {
                            break;
                        } //                        ExecutorState::RunningExclusive => {}
                    }
                }
                ExecutorState::Running => {}
                ExecutorState::Exit => {
                    break;
                } /*                ExecutorState::RunningExclusive => {
                   *                    while self.try_steal_and_run() {
                   *                        self.poll_futures().await;
                   *                    }
                   *                    self.await_all_futures().await;
                   *                    self.shared.exclusive_finished.notified().await;
                   *                } */
            }
            if let Some(mut current_access) = self.shared.access.try_lock() {
                let queue = self.shared.enqueued[self.id].lock();
                let some_enqueued = self.enq_systems(&mut *current_access).await;
                drop(current_access);
                if !some_enqueued {
                    if !self.try_steal_and_run() {
                        self.shared.system_finished.notified().await;
                    }
                }
            } else {
                if !self.try_steal_and_run() {
                    self.shared.system_finished.notified().await;
                }
            }
        }
    }

    async fn poll_futures(&mut self, state: Option<&mut SharedAccess>) {
        if self.async_systems.is_empty() {
            return;
        }
        zmaxion_rt::yield_now().await;
        match state {
            None => {
                let state = self.shared.access.lock();
                while let Some(Some(id)) = self.async_systems.next().now_or_never() {
                    self.system_finished(id, &mut *state);
                }
            }
            Some(state) => {
                while let Some(Some(id)) = self.async_systems.next().now_or_never() {
                    self.system_finished(id, state);
                }
            }
        }
    }

    async fn pause(&mut self) {
        loop {
            if !self.try_steal_and_run() {
                break;
            }
        }
        self.await_all_futures().await;
        if self.shared.paused_workers.fetch_add(1, Relaxed) == self.shared.n_workers() - 1 {
            self.shared.paused_workers.fetch_and(0, Relaxed);
            self.shared.executor_flushed.notify_waiters();
            self.shared.start_running.notified().await;
        }
    }

    fn try_steal_and_run(&mut self) -> bool {
        let start = rand::random::<usize>() % self.shared.n_workers();
        for i in start..start + self.shared.n_workers() {
            let i = i % self.shared.n_workers();
            if i == self.id {
                continue;
            }
            if let Some(id) = self.shared.enqueued[i].lock().pop_front() {
                self.run_system(id, None);
                return true;
            }
        }
        false
    }

    async fn await_all_futures(&mut self) {
        while let Some(id) = self.async_systems.next().await {
            let mut access = self.shared.access.lock();
            self.shared.system_finished(id, &mut *access);
        }
    }

    async fn run_exclusive_systems(&mut self, state: &mut SharedAccess) {
        self.shared.state.store(ExecutorState::Pausing, SeqCst);
        self.await_all_futures().await;
        self.shared.executor_flushed.notified().await;

        state.waiting_for_access_primary.retain(|id| {
            if !self.shared.systems.needed_access[*id].exclusive_access {
                return true;
            }
            self.run_system::<false>(*id);
            false
        });
        state.waiting_for_access_secondary.retain(|id| {
            if !self.shared.systems.needed_access[id.id].exclusive_access {
                return true;
            }
            self.run_system::<false>(id.id);
            false
        });
        self.shared.start_running.notify_waiters();
        self.shared.system_finished.notify_waiters();
        self.shared.exclusive_finished.notify_waiters();
    }

    async fn enq_systems(&mut self, state: &mut SharedAccess) -> bool {
        let mut run_exclusive_systems = false;
        let mut enqueued = false;
        fn enq_non_exclusive(
            shared: &WorkerShared,
            id: SystemId,
            active: &mut [ActiveWorldAccess],
            enqueued: &mut bool,
            on_compatible: impl FnMut(),
        ) {
            let mut compatible = true;
            for needed in &shared.systems.needed_access[id].by_world {
                if !needed.access.is_compatible(&active[needed.world_id]) {
                    compatible = false;
                    break;
                }
            }
            if compatible {
                for needed in &shared.systems.needed_access[id].by_world {
                    add_access(active, needed);
                }
                shared.enqueued[id].lock().push_back(id);
                shared.system_finished.notify_one();
                on_compatible();
                *enqueued = true;
            }
        }
        if !self.shared.waiting_for_access_primary.is_empty() {
            let mut i = 0;
            while i < self.shared.waiting_for_access_primary.len() {
                let id = self.shared.waiting_for_access_primary[i];
                if self.shared.systems.needed_access[id].exclusive_access {
                    run_exclusive_systems = true;
                    break;
                } else {
                    enq_non_exclusive(&self.shared, id, &mut state.access, &mut enqueued, || {
                        state.waiting_for_access_primary.swap_remove(i);
                        i -= 1;
                    });
                }
                i += 1;
            }
            return enqueued;
        }
        if run_exclusive_systems {
            self.run_exclusive_systems(state).await;
        }
        state.waiting_for_access_secondary.retain(|node| {
            if run_exclusive_systems {
                return true;
            }
            let id = node.id;
            if self.shared.systems.needed_access[id].exclusive_access {
                run_exclusive_systems = true;
                return true;
            } else {
                let mut compatible = false;
                enq_non_exclusive(&self.shared, id, &mut state.access, &mut enqueued, || {
                    compatible = true;
                });
                !compatible
            }
        });
        if run_exclusive_systems {
            self.run_exclusive_systems(state).await;
            false
        } else {
            enqueued
        }
    }

    fn run_system(&mut self, id: SystemId, state: Option<&mut SharedAccess>) {
        let mut context = ExecutorContext {
            real_time_runtime: &self.shared.real_time_runtime,
            worker_runtime_handle: &self.handle,
            futures: &self.async_systems,
            system_id: id,
        };
        unsafe {
            let system = &mut self.shared.systems.systems[id as usize];
            system.system.run_unsafe(&mut context);
            match system.system_kind {
                SystemKind::Compute => match state {
                    None => {
                        let state = self.shared.access.lock();
                        self.shared.system_finished(id, &mut *state);
                    }
                    Some(state) => {
                        self.shared.system_finished(id, state);
                    }
                },
                SystemKind::Async => {}
            }
        }
    }
}

impl WorkerShared {
    fn system_finished(&self, id: SystemId, active: &mut SharedAccess) {
        let system = &mut self.systems.systems[id as usize];

        self.systems.positions[id as usize].store(system.max_position(), SeqCst);
        active.enqueued.toggle(id);
        let needed_access = self.systems.needed_access[id];
        for access in needed_access.by_world {
            for id in &access.access.reads {
                active.access[access.world_id].n_reads[id.index()] -= 1;
            }
            for id in &access.access.writes {
                active.access[access.world_id].writes[id.index()] = false;
            }
        }

        for id in system.dependant_systems() {
            if self.systems.positions[*id as usize].fetch_sub(1, Relaxed) == 1 {
                self.submit_system(*id, active);
            }
        }
        for id in system.notify_systems() {
            self.submit_system(*id, active);
        }
        for id in system.recurisve_prev_jobs_with_ambigous_access() {
            self.systems.positions[*id as usize].fetch_sub(1, Relaxed);
        }
    }

    fn submit_system(&mut self, id: SystemId, access: &mut SharedAccess) {
        if access.enqueued.contains(id as usize) {
            return;
        }
        access.enqueued.insert(id as usize);
        if self.systems.needed_access[id as usize].exclusive_access {
            self.waiting_for_access_primary_sender
                .send(PrioritySystem::Prebuilt(id))
                .ignore();
        } else {
            access.waiting_for_access_secondary.push_back(SystemToRun {
                generation: self.generation.load(SeqCst),
                id,
            });
        }
    }
}

// async fn run_exclusive_systems(state: &mut SharedAccess,
// shared: &WorkerShared,
//
//) {
//    self.shared.state.store(ExecutorState::Pausing, SeqCst);
//    self.await_all_futures().await;
//    self.shared.executor_flushed.notified().await;
//
//    state.waiting_for_access_primary.retain(|id| {
//        if !self.shared.systems.needed_access[*id].exclusive_access {
//            return true;
//        }
//        self.run_system::<false>(*id);
//        false
//    });
//    state.waiting_for_access_secondary.retain(|id| {
//        if !self.shared.systems.needed_access[id.id].exclusive_access {
//            return true;
//        }
//        self.run_system::<false>(id.id);
//        false
//    });
//    self.shared.start_running.notify_waiters();
//    self.shared.system_finished.notify_waiters();
//    self.shared.exclusive_finished.notify_waiters();
//}
// fn enq_systems(queue: &mut VecDeque<SystemId>, state: &mut SharedAccess,
// waiting_for_access_primary: &mpsc::Receiver<PrioritySystem>,
//    shared: &WorkerShared,
//) -> bool {
//         let mut run_exclusive_systems = false;
//        let mut enqueued = false;
//        fn enq_non_exclusive(
//            shared: &WorkerShared,
//            id: SystemId,
//            active: &mut [ActiveWorldAccess],
//            enqueued: &mut bool,
//            on_compatible: impl FnMut(),
//        ) {
//            let mut compatible = true;
//            for needed in &shared.systems.needed_access[id].by_world {
//                if !needed.access.is_compatible(&active[needed.world_id]) {
//                    compatible = false;
//                    break;
//                }
//            }
//            if compatible {
//                for needed in &shared.systems.needed_access[id].by_world {
//                    add_access(active, needed);
//                }
//                shared.enqueued[id].lock().push_back(id);
//                shared.system_finished.notify_one();
//                on_compatible();
//                *enqueued = true;
//            }
//        }
//        if !waiting_for_access_primary.is_empty() {
//            let mut i = 0;
//            while i < waiting_for_access_primary.len() {
//                let id = waiting_for_access_primary[i];
//                if shared.systems.needed_access[id].exclusive_access {
//                    run_exclusive_systems = true;
//                    break;
//                } else {
//                    enq_non_exclusive(shared, id, &mut state.access, &mut enqueued, || {
//                        state.waiting_for_access_primary.swap_remove(i);
//                        i -= 1;
//                    });
//                }
//                i += 1;
//            }
//            return enqueued;
//        }
//        if run_exclusive_systems {
//            self.run_exclusive_systems(state).await;
//        }
//        state.waiting_for_access_secondary.retain(|node| {
//            if run_exclusive_systems {
//                return true;
//            }
//            let id = node.id;
//            if self.shared.systems.needed_access[id].exclusive_access {
//                run_exclusive_systems = true;
//                return true;
//            } else {
//                let mut compatible = false;
//                enq_non_exclusive(shared, id, &mut state.access, &mut enqueued, || {
//                    compatible = true;
//                });
//                !compatible
//            }
//        });
//        if run_exclusive_systems {
//            self.run_exclusive_systems(state).await;
//            false
//        } else {
//            enqueued
//        }
//}

fn add_access(active: &mut [ActiveWorldAccess], needed: &NeededWorldAccess) {
    for id in needed.access.reads {
        active[needed.world_id].n_reads[id.index()] += 1;
    }
    for id in needed.access.writes {
        active[needed.world_id].writes[id.index()] = true;
    }
}

#[derive(Clone, Copy)]
enum ExecutorState {
    Pausing,
    //    Paused,
    Running,
    //    RunningExclusive,
    //    RunningFuturesOnly,
    Exit,
}

pub struct WorkerShared {
    /// How many systems can run before this system has to run.
    /// Used to prevent starvation, eg. in a queue there is exclusive system and many that read
    /// components. Exclusive system would never get to run if executor always has work.
    /// If time to execute gets exceeded then then systems move to primary queue
    time_to_execute_threshold: u64,
    waiting_for_access_primary: mpsc::Receiver<PrioritySystem>,
    waiting_for_access_primary_sender: mpsc::Sender<PrioritySystem>,
    access: Mutex<SharedAccess>,
    start_running: Notify,
    systems: Systems,
    enqueued: Vec<Mutex<VecDeque<SystemId>>>,
    state: Atomic<ExecutorState>,
    paused_workers: AtomicUsize,
    executor_flushed: Notify,
    finished_ids: Mutex<Vec<SystemId>>,
    system_finished: Notify,
    exclusive_finished: Notify,
    real_time_runtime: Runtime,
    generation: Atomic<SystemId>,
}

impl WorkerShared {
    fn n_workers(&self) -> usize {
        self.enqueued.len()
    }
}

pub struct SharedAccess {
    enqueued: FixedBitSet,
    waiting_for_access_secondary: VecList<SystemToRun, SystemId>,
    access: Vec<ActiveWorldAccess>,
}

impl SharedAccess {
    fn enq(&mut self, shared: &WorkerShared, id: SystemId) {
    }
}

struct ActiveWorldAccess {
    n_reads: Vec<SystemId>,
    writes: FixedBitSet,
}

impl ActiveWorldAccess {
    fn remove(&self, other: Access) {
        todo!()
    }
}

pub struct SystemToRun {
    generation: SystemId,
    id: SystemId,
}

pub struct NeededAccess {
    exclusive_access: bool,
    by_world: Vec<NeededWorldAccess>,
}

pub struct SystemData {
    system_kind: SystemKind,
    system: Box<dyn System>,
    // contains:
    // dependants: Vec<SystemId>,
    // recurisve_prev_jobs_with_ambigous_access: Vec<SystemId>,
    n_dependants: SystemId,
    n_notify_systems: SystemId,
    n_recurisve_prev_jobs_with_ambigous_access: SystemId,
    linked_systems: Vec<SystemId>,
}

impl SystemData {
    fn dependant_systems(&self) -> &[SystemId] {
        &self.linked_systems[0..self.n_dependants as usize]
    }

    fn notify_systems(&self) -> &[SystemId] {
        let start = self.n_dependants as usize;
        &self.linked_systems[start..start + self.n_notify_systems as usize]
    }

    fn recurisve_prev_jobs_with_ambigous_access(&self) -> &[SystemId] {
        let start = self.n_dependants as usize + self.n_notify_systems as usize;
        &self.linked_systems
            [start..start + self.n_recurisve_prev_jobs_with_ambigous_access as usize]
    }

    fn max_position(&self) -> SystemId {
        self.n_dependants + self.n_recurisve_prev_jobs_with_ambigous_access
    }
}

pub struct Systems {
    needed_access: Vec<NeededAccess>,
    positions: Vec<Atomic<SystemId>>,
    systems: Vec<SystemData>,
    free_form_systems: Vec<SystemId>,
    current_free_form_system: AtomicUsize,
}

pub struct NeededWorldAccess {
    world_id: WorldId,
    access: Access,
    //    reads_and_writes: FixedBitSet,
    //    writes: FixedBitSet,
}

pub enum PrioritySystem {
    Prebuilt(SystemId),
    Custom(SystemContainer),
}

pub struct SystemContainer {
    kind: SystemKind,
    access: NeededAccess,
    system: Box<dyn System>,
    finished: SystemData,
}

pub struct ExecutorContext<'a> {
    real_time_runtime: &'a Runtime,
    worker_runtime_handle: &'a Handle,
    futures: &'a FuturesUnordered<SystemFuture>,
    system_id: SystemId,
}

#[pin_project::pin_project]
pub struct SystemFuture {
    id: SystemId,
    #[pin]
    handle: JoinHandle<()>,
}

impl Future for SystemFuture {
    type Output = SystemId;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        ready!(me.handle.poll(cx));
        Poll::Ready(self.id)
    }
}

impl<'a> ExecutorContext<'a> {
    // TODO: Remove send requirement
    pub fn spawn_local(&self, fut: impl Future<Output = ()> + Send) {
        let handle = self.worker_runtime_handle.spawn(fut).into_inner();
        self.futures.push(SystemFuture {
            id: self.system_id,
            handle,
        });
    }

    pub fn spawn(&self, fut: impl Future<Output = ()> + Send) {
        let handle = self.real_time_runtime.spawn(fut).into_inner();
        self.futures.push(SystemFuture {
            id: self.system_id,
            handle,
        });
    }
}

pub trait System {
    unsafe fn run_unsafe<'a>(&mut self, context: &mut ExecutorContext<'a>);
}

enum SystemKind {
    Compute,
    Async,
}

pub struct Notification {
    archetype_id: ArchetypeId,
}

type WorldId = usize;

// add system and update all dependencies
// remove system and update all referenced
// swap boxed system
// add/remove dependencies
// run system by id

// load plugin
// load all dependency plugins
// add all systems to schedule, exclusive system, can add other things
// run init systems in parallel
// once all have run, emit plugin loaded event
// on loaded event remove load systems from schedule and start loading depentandts

// unload plugin
// add unload systems to schedule
// once finished remove them, then run exclusive system, then unload dependency if no dependants

pub struct Schedule {
    shared_among_workers: Arc<WorkerShared>,
}

struct Access {
    reads: Vec<ArchetypeComponentId>,
    writes: Vec<ArchetypeComponentId>,
}

impl Access {
    pub(crate) fn is_compatible(&self, active: &ActiveWorldAccess) -> bool {
        for id in &self.reads {
            if active.writes[id.index()] {
                return false;
            }
        }
        for id in &self.writes {
            if active.n_reads[id.index()] != 0 {
                return false;
            }
        }
        true
    }
}
