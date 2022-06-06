use async_trait::async_trait;
use zmaxion_core::{models::TopicRef, prelude::*};
use zmaxion_param::{
    serde::de::DeserializeOwned, ControlFlow, ParamBuilder, PipeObserver, PipeParam,
    PipeParamFetch, PipeParamImpl, PipeParamState, PipeParamStateImpl, TopicParam, TopicParamKind,
};
use zmaxion_utils::prelude::*;

use crate::*;

pub trait TopicImpl<'i, T> {
    type Reader: TopicReaderImpl<'i, T>;
    //    type Writer: TopicWriterImpl<T>;
    const FEATURES: TopicFeatures;
}

#[async_trait]
pub trait TopicReaderImpl<'i, T> {
    type State: TopicReaderStateImpl<'i, T>;
    async fn read<'a>(&'a mut self) -> ReadGuard<'a, T>;
    fn try_read<'a>(&'a mut self) -> ReadGuard<'a, T>;
}

#[async_trait]
pub trait TopicReaderStateImpl<'s, T>: Sized {
    type Source: Component + Clone;
    type Args: DeserializeOwned + Send + 'static;
    type Param: TopicReaderImpl<'s, T>;

    fn new_rw(topic: Self::Source) -> AnyResult<Self>;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self> {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world();
        let topic = world.get::<Self::Source>(id).unwrap();
        Self::new_rw(topic.clone())
    }

    #[inline]
    fn should_run(&mut self) -> bool {
        true
    }

    fn get_param(&'s mut self) -> Self::Param;

    /// Called before the pipe gets executed
    fn pipe_executing(&mut self) {
    }
    /// Called after the pipe has executed
    fn pipe_executed<R>(&mut self, result: &AnyResult<R>) -> ControlFlow {
        ControlFlow::default()
    }
}

#[async_trait]
pub trait TopicWriterImpl<'i, T> {
    type State: TopicWriterStateImpl<'i, T>;

    fn write_slice(&mut self, data: &[T])
    where
        T: Clone;
    fn write(&mut self, value: T);
}

#[async_trait]
pub trait TopicWriterStateImpl<'s, T>: Sized {
    type Source: Component + Clone;
    type Args: DeserializeOwned + Send + 'static;
    type Param: TopicWriterImpl<'s, T>;

    fn new_rw(topic: Self::Source) -> AnyResult<Self>;

    async fn new(builder: ParamBuilder<Self::Args>) -> AnyResult<Self> {
        let id = builder.get_topic_id(Self::type_name())?;
        let world = builder.world();
        let topic = world.get::<Self::Source>(id).unwrap();
        Self::new_rw(topic.clone())
    }

    #[inline]
    fn should_run(&mut self) -> bool {
        true
    }

    fn get_param(&'s mut self) -> Self::Param;

    /// Called before the pipe gets executed
    fn pipe_executing(&mut self) {
    }
    /// Called after the pipe has executed
    fn pipe_executed<R>(&mut self, result: &AnyResult<R>) -> ControlFlow {
        ControlFlow::default()
    }
}

#[macro_export]
macro_rules! impl_topic_reader {
    (
        $life:lifetime,
        $event:ty,
        [$($generics:tt)*],
        $param:ty
    ) => {
        impl<$($generics)*> $crate::__export__::ParamImpl for $param {
            type State = <$param as TopicReaderImpl<$life, $event>>::State;
        }
        $crate::__export__::impl_param!($life, [$($generics)*], $param);
    };
}

#[macro_export]
macro_rules! impl_topic_reader_state {
    (
        $life:lifetime,
        $event:ty,
        [$($generics:tt)*],
        $state:ty
    ) => {
        $crate::__export__::impl_param_state!($life, [$($generics)*], <$state as $crate::TopicReaderImpl<$life, $event>>::State);
        $crate::impl_topic_reader_state!(@life $life, [$($generics)*], $state, $event);
    };
    (@life $life:lifetime, [$($generics:tt)+], $state:ty, $event:ty
    ) => {
        #[$crate::__export__::async_trait]
        impl<$($generics)+> ParamStateImpl<$life> for <$state as $crate::TopicReaderImpl<$life, $event>>::State {
            type Args = <$state as $crate::TopicReaderStateImpl<$life, $event>>::Args;
            type Param = $state;
            type Source = <$state as TopicReaderStateImpl<$life, $event>>::Source;

            const KIND: $crate::__export__::TopicParamKind = $crate::__export__::TopicParamKind::Reader;

            fn new_rw(topic: Self::Source) -> $crate::__export__::AnyResult<Self> {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::new_rw(topic)
            }

            async fn new(builder: $crate::__export__::ParamBuilder<Self::Args>) -> $crate::__export__::AnyResult<Self>
            where
                Self: Sized,
            {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::new(builder).await
            }

            fn get_param(&$life mut self) -> Self::Param {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::get_param(self)
            }

            fn should_run(&mut self) -> bool {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::should_run(self)
            }

            fn topic_param(&self) -> Option<$crate::__export__::TopicParam> {
                Some($crate::__export__::TopicParam::Reader($crate::__export__::Entity::from_raw(u32::MAX)))
            }

            fn pipe_executing(&mut self) {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::pipe_executing(self)
            }

            fn pipe_executed<R>(&mut self, result: &$crate::__export__::AnyResult<R>) -> $crate::__export__::ControlFlow {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::pipe_executed(self, result)
            }
        }
    };
    (@life $life:lifetime, [], $state:ty, $event:ty
    ) => {
        #[$crate::__export__::async_trait]
        impl<$life> ParamStateImpl<$life> for <$state as $crate::TopicReaderImpl<$life, $event>>::State {
            type Args = <$state as $crate::TopicReaderImpl<$life>, $event>::Args;
            type Param = <$state as TopicReaderStateImpl<$life, $event>>::Param;
            type Source = <$state as TopicReaderStateImpl<$life, $event>>::Source;

            const KIND: $crate::__export__::TopicParamKind = $crate::__export__::TopicParamKind::Reader;

            fn new_rw(topic: Self::Source) -> $crate::__export__::AnyResult<Self> {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::new_rw(topic)
            }

            async fn new(builder: $crate::__export__::ParamBuilder<Self::Args>) -> $crate::__export__::AnyResult<Self>
            where
                Self: Sized,
            {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::new(builder).await
            }

            fn get_param(&'s mut self) -> Self::Param {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::get_param(self)
            }

            fn should_run(&mut self) -> bool {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::should_run(self)
            }

            fn topic_param(&self) -> Option<$crate::__export__::TopicParam> {
                Some($crate::__export__::TopicParam::Reader($crate::__export__::Entity::from_raw(u32::MAX)))
            }

            fn pipe_executing(&mut self) {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::pipe_executing(self)
            }

            fn pipe_executed<T>(&mut self, result: &$crate::__export__::AnyResult<T>) -> $crate::__export__::ControlFlow {
                <$state as $crate::TopicReaderStateImpl<$life, $event>>::pipe_executed(self, result)
            }
        }
    }
}

#[macro_export]
macro_rules! impl_topic_writer {
    (
        $life:lifetime,
        $event:ty,
        [$($generics:tt)*],
        $param:ty
    ) => {
        impl<$($generics)*> $crate::__export__::ParamImpl for $param {
            type State = <$param as TopicWriterImpl<$life, $event>>::State;
        }
        $crate::__export__::impl_param!($life, [$($generics)*], $param);
    };
}

#[macro_export]
macro_rules! impl_topic_writer_state {
    (
        $life:lifetime,
        $event:ty,
        [$($generics:tt)*],
        [$($lifetime_and_generics:tt)*],
        $state:ty
    ) => {
        $crate::impl_topic_writer_state!(@life $life, [$($generics)*], $state, $event);
        $crate::__export__::impl_param_state!($life, [$($lifetime_and_generics)*], <$state as $crate::TopicWriterImpl<$life, $event>>::State);
    };
    (@life $life:lifetime, [$($lifetime_and_generics:tt)+], $state:ty, $event:ty
    ) => {
        #[$crate::__export__::async_trait]
        impl<$($lifetime_and_generics)+> $crate::__export__::ParamStateImpl<$life> for $state {
            type Args = <$state as $crate::TopicWriterStateImpl<$life, $event>>::Args;
            type Param = <$state as $crate::TopicWriterStateImpl<$life, $event>>::Param;
            type Source = <$state as $crate::TopicWriterStateImpl<$life, $event>>::Source;

            const KIND: $crate::__export__::TopicParamKind = $crate::__export__::TopicParamKind::Writer;

            fn new_rw(topic: Self::Source) -> $crate::__export__::AnyResult<Self> {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::new_rw(topic)
            }

            async fn new(builder: $crate::__export__::ParamBuilder<Self::Args>) -> $crate::__export__::AnyResult<Self>
            where
                Self: Sized,
            {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::new(builder).await
            }

            fn get_param(&$life mut self) -> Self::Param {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::get_param(self)
            }

            fn should_run(&mut self) -> bool {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::should_run(self)
            }

            fn topic_param(&self) -> Option<$crate::__export__::TopicParam> {
                Some($crate::__export__::TopicParam::Writer($crate::__export__::Entity::from_raw(u32::MAX)))
            }

            fn pipe_executing(&mut self) {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::pipe_executing(self)
            }

            fn pipe_executed<R>(&mut self, result: &$crate::__export__::AnyResult<R>) -> $crate::__export__::ControlFlow {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::pipe_executed(self, result)
            }
        }
        impl<$($lifetime_and_generics)*> Extend<$event> for <$state as $crate::TopicWriterImpl<$life, $event>>::State {
            fn extend<__I: IntoIterator<Item = $event>>(&mut self, iter: __I) {
                for value in iter.into_iter() {
                    // iterators could take a long time to yield, blocking for each item so that
                    // other writers could progress
                    self.write(value)
                }
            }
        }
    };
    (@life $life:lifetime, [], $state:ty, $event:ty
    ) => {
        #[$crate::__export__::async_trait]
        impl<$life> ParamStateImpl<$life> for <$state as $crate::TopicWriterImpl<$life, $event>>::State {
            type Args = <$state as $crate::TopicWriterStateImpl<$life, $event>>::Args;
            type Param = <$state as $crate::__export__::TopicWriterStateImpl<$life, $event>>::Param;
            type Source = <$state as $crate::__export__::TopicWriterStateImpl<$life, $event>>::Source;

            const KIND: $crate::__export__::TopicParamKind = $crate::__export__::TopicParamKind::Writer;

            fn new_rw(topic: Self::Source) -> $crate::__export__:AnyResult<Self> {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::new_rw(topic)
            }

            async fn new(builder: $crate::__export__::ParamBuilder<Self::Args>) -> $crate::__export__::AnyResult<Self>
            where
                Self: Sized,
            {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::new(builder).await
            }

            fn get_param(&'s mut self) -> Self::Param {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::get_param(self)
            }

            fn should_run(&mut self) -> bool {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::should_run(self)
            }

            fn topic_param(&self) -> Option<$crate::__export__::TopicParam> {
                Some($crate::__export__::TopicParam::Writer($crate::__export__::Entity::from_raw(u32::MAX)))
            }

            fn pipe_executing(&mut self) {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::pipe_executing(self)
            }

            fn pipe_executed<T>(&mut self, result: &$crate::__export__::AnyResult<T>) -> $crate::__export__::ControlFlow {
                <$state as $crate::TopicWriterStateImpl<$life, $event>>::pipe_executed(self, result)
            }
        }
        impl<$life> Extend<$event> for <$state as $crate::TopicWriterImpl<$life, $event>>::State {
            fn extend<__I: IntoIterator<Item = $event>>(&mut self, iter: __I) {
                for value in iter.into_iter() {
                    // iterators could take a long time to yield, blocking for each item so that
                    // other writers could progress
                    self.write(value)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use async_trait::async_trait;
    use zmaxion_core::bevy::ecs::system::Resource;

    use crate::{
        topic_impl::{
            TopicReaderImpl, TopicReaderStateImpl, TopicWriterImpl, TopicWriterStateImpl,
        },
        ReadGuard,
    };

    struct Reader<'a, T>(PhantomData<&'a T>);
    struct Writer<'a, T>(PhantomData<&'a T>);
    struct ReaderState<T>(PhantomData<T>);
    struct WriterState<T>(PhantomData<T>);
    struct CustomTopic<T>(PhantomData<T>);

    impl_topic_writer!(['s, T: Resource], Writer<'s, T>, T);
    impl_topic_reader!(['s, T: Resource], Reader<'s, T>, T);
    impl_topic_writer_state!([T: Resource], WriterState<T>, T);
    impl_topic_reader_state!([T: Resource], ReaderState<T>, T);

    impl<'s, T: Resource> TopicWriterImpl<'s, T> for Writer<'s, T> {
        type State = WriterState<T>;

        fn write_slice<'a>(&'a mut self, data: &[T])
        where
            T: Clone,
        {
        }

        fn write<'a>(&'a mut self, value: T) {
        }
    }

    #[async_trait]
    impl<'s, T: Resource> TopicReaderImpl<'s, T> for Reader<'s, T> {
        type State = ReaderState<T>;

        async fn read<'a>(&'a mut self) -> ReadGuard<'a, T> {
            unimplemented!()
        }

        fn try_read<'a>(&'a mut self) -> ReadGuard<'a, T> {
            unimplemented!()
        }
    }

    impl<'s, T: Resource> TopicReaderStateImpl<'s, T> for ReaderState<T> {
        type Args = ();
        type Param = Reader<'s, T>;
        type Source = CustomTopic<T>;

        fn new_rw(topic: Self::Source) -> zmaxion_utils::prelude::AnyResult<Self> {
            unimplemented!()
        }

        fn get_param(&'s mut self) -> Self::Param {
            unimplemented!()
        }
    }

    impl<'s, T: Resource> TopicWriterStateImpl<'s, T> for WriterState<T> {
        type Args = ();
        type Param = Writer<'s, T>;
        type Source = CustomTopic<T>;

        fn new_rw(topic: Self::Source) -> zmaxion_utils::prelude::AnyResult<Self> {
            unimplemented!()
        }

        fn get_param(&'s mut self) -> Self::Param {
            unimplemented!()
        }
    }
}
