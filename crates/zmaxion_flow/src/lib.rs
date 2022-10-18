#![deny(unused_must_use)]
use zmaxion_utils::pool::{PoolArc, PoolItem};

pub mod default;
pub mod flow;
pub mod flower;

pub mod prelude {
    pub use crate::flower::Flower;
}

pub struct Event {
    pub data: PoolArc<dyn PoolItem>,
    pub row: u32,
}

pub struct EventsAll {
    pub data: PoolArc<dyn PoolItem>,
}

pub struct EventRange {
    pub data: PoolArc<dyn PoolItem>,
    pub start: u32,
    pub end: u32,
}
