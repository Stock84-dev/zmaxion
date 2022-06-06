use std::sync::{atomic::AtomicU64, Arc};

use zmaxion_core::prelude::HashMap;

use crate::dyn_topic::{DynReader, DynWriter};

struct A;
impl<T> Reader<T> for A {
    fn next(&mut self) -> Option<T> {
        todo!()
    }
}

pub trait Writer<T> {}

pub struct Meta<T: ?Sized> {
    generation: Vec<AtomicU64>,
    data: T,
}

impl<T> Iterator for dyn Reader<T> {
    type Item = Meta<T>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

struct Scheduler {
    jobs: HashMap<PipelineId, Vec<Box<dyn Job>>>,
    topics: HashMap<TopicName, Topic>,
    pipelines: Vec<Pipeline>,
}

struct Pipeline {
    readers: Vec<Arc<dyn AnyReader>>,
    ack_messages: HashMap<(ReaderId, u64), AtomicU64>,
    events: Arena,
}

fn a() {
    let mut a: Box<dyn Reader<()>> = Box::new(A);
    //    a.next();
    let mut b = a;
    b.next();
    let m = Meta {
        generation: 0,
        data: (),
    };
    let c: Box<Meta<dyn Reader<()>>> = Box::new(Meta {
        generation: 0,
        data: A,
    });
}
