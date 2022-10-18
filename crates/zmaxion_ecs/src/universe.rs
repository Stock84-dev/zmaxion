use crate::{
    data_types::SparseVec,
    parallel_executor::ParallelExecutor,
    storage::SparseArray,
    world::{World, WorldId},
};

pub struct Universe {
    worlds: SparseVec<World>,
    executor: Option<ParallelExecutor>,
}

impl Universe {
    pub fn run(&mut self) {
        match &mut self.executor {
            None => panic!("No executor present"),
            Some(executor) => {
                executor.run(&mut self.worlds);
            }
        }
    }
}
