use std::marker::PhantomData;

use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Dummy<T>(PhantomData<T>);
impl<T> Default for Dummy<T> {
    fn default() -> Self {
        Dummy(Default::default())
    }
}
pub struct DummyReader<T>(PhantomData<T>);
pub struct DummyWriter<T>(PhantomData<T>);

pub mod prelude {
    pub use crate::{DummyReader, DummyWriter};
}

#[macro_export]
macro_rules! __reader_state_type {
    () => {
        $crate::DummyReader<T>
    };
}

#[macro_export]
macro_rules! __writer_state_type {
    () => {
        $crate::DummyWriter<T>
    };
}

#[macro_export]
macro_rules! __default_topic {
    () => {
        $crate::Dummy::<T>::default()
    };
}
