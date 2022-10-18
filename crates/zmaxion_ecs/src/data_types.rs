use std::ops::{Deref, DerefMut};

pub struct SparseVec<T>(Vec<Option<T>>);

impl<T> SparseVec<T> {
    pub fn new() -> Self {
        Self { 0: vec![] }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            0: Vec::with_capacity(capacity),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index)?.as_ref()
    }
}

