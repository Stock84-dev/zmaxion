use std::{
    convert::Infallible,
    future,
    future::{Future, Ready},
};

use bevy_utils::define_label;
use futures_util::FutureExt;
// things to consider:
// message can have different type based on a param when compiling a template
// because messagaes are heavy we should avoid checking what message type there is
// to do that we need to box the graph, it is fine
// if message combination 1
//      if has params[0]
//          match params[0] {
//              concrete0 => if has params[1]
//                  match params[1]
//                      let boxed_graph = Box::new(Graph::<...>::new());

// to save us an implementation for every combinator we wrap a stream an use theirs
// some methods on stream must not be called like zip and cloned, that wy we need to wrap

// error scenarios:
// send to dead letter mailbox if out of order execution
// trace back to presistent topics and states of a pipeline and resend an event
// do nothing
// despawn pipe/pipeline
// restart pipe/pipeline
// exponential backoff

pub mod templates {
    use std::marker::PhantomData;

    use zmaxion_core::{
        models::{DynPipeFeatures, PipelineFeatures},
        prelude::Entity,
    };
    use zmaxion_topic::{prelude::TopicWriter, TopicFeatures};
    use zmaxion_topics::Writer;
    use zmaxion_utils::convert::IntoTryFuture;

    use crate::{nodes, nodes::PipeGraphNode};

    pub trait Message: Sized {
        type Data;
        fn into_message(self) -> Self::Data;
        fn message(&self) -> &Self::Data;
        fn join<O: Message>(self, other: O) -> (Self, O);
        //        fn map<T, U: Message<T>>(&self, mapped: T) -> U;
        fn fork(&self) -> Self;
    }

    pub struct AckData {
        topic: Entity,
        message_id: u32,
        generation: u32,
    }

    pub type Msg<T> = MaybeAckedMessage<MaybeTracedMessage<T>>;

    impl<T> Message for Msg<T> {
        type Data = T;

        fn into_message(self) -> Self::Data {
            todo!()
        }

        fn message(&self) -> &Self::Data {
            todo!()
        }

        fn join<O: Message>(self, other: O) -> (Self, O) {
            todo!()
        }

        fn fork(&self) -> Self {
            todo!()
        }
    }

    pub struct AckedMessage<T> {
        inner: T,
        acks: Vec<AckData>,
    }

    pub enum MaybeAckedMessage<T> {
        None(T),
        Acked(AckedMessage<T>),
    }

    pub struct TraceData {
        pipeline: Entity,
        pipe: Entity,
        topic: Entity,
        produced_ts: i64,
        param_pos: u8,
    }

    pub enum TraceEntry {
        Single(TraceData),
        Join(Vec<TraceData>),
    }

    pub struct Features<'a> {
        pipe: &'a DynPipeFeatures,
        topic: &'a TopicFeatures,
        pipeline: &'a PipelineFeatures,
        trace: bool,
    }

    pub trait PipeParamName {
        fn name(&self) -> String;
    }

    pub struct TracedMessage<T> {
        inner: T,
        path: Vec<TraceEntry>,
    }

    pub enum MaybeTracedMessage<T> {
        None(T),
        Traced(TracedMessage<T>),
    }

    // impl template for any label
    pub trait PipeGraphTemplate<C: MessageConverter<Self::Data>>: Sized {
        type Node: PipeGraphNode;
        type Params;
        type Data;
        fn into_node(self, features: &Features) -> Self::Node;
        fn filter<Fm, Rm, F: FnMut(C) -> bool>(self, f: F) -> Filter<Self, F, Fm, Rm> {
            Filter {
                template: self,
                f,
                _t: Default::default(),
            }
        }
        fn map<F: FnMut(C::Message) -> T, T>(self, f: F) -> Map<Self, F, T> {
            Map {
                template: self,
                f,
                _t: Default::default(),
            }
        }
        fn fork<F: FnOnce(Branch<C>) -> T, T: PipeGraphTemplate<C>>(self, f: F) -> Fork<Self, T> {
            Fork {
                main: self,
                branch: f(Branch {
                    _t: Default::default(),
                }),
            }
        }
        fn write<N: PipeParamName>(
            self,
            name: N,
        ) -> Write<Self, (Self::Params, Writer<Self::Data>)> {
            let name = name.name();
            Write {
                template: self,
                writer: name,
                _t: PhantomData,
            }
        }
    }

    pub struct Filter<T, F, C, Params> {
        template: T,
        f: F,
        _t: PhantomData<Params>,
    }

    pub struct Map<T, F, R> {
        template: T,
        f: F,
        _t: PhantomData<R>,
    }

    pub struct Fork<Main, Branch> {
        main: Main,
        branch: Branch,
    }

    pub struct Branch<Item> {
        _t: PhantomData<Item>,
    }

    pub struct Write<T, Params> {
        template: T,
        writer: String,
        _t: PhantomData<Params>,
    }

    pub trait MessageConverter<D> {
        type Message;

        fn convert_into_message(&mut self, data: D) -> Self::Message;
        fn convert_to_value<M: Message>(&mut self, m: &M) -> M::Data;
    }

    impl<C, T: PipeGraphTemplate<C>, F, R, Fm, Rm> PipeGraphTemplate<C> for Filter<T, F, Fm, Rm>
    where
        F: FnMut(&T::Data) -> R,
        R: IntoTryFuture<Fm, Rm>,
    {
        type Data = T::Data;
        type Node = nodes::Filter<T::Node, F, Fm, Rm>;

        fn into_node(self, features: &Features) -> Self::Node {
            panic!()
            //            let node = self.template.into_node(features);
            //            let f = |item| async move {
            //                self.f()
            //                item.into_try_future()
            //            };
            //            nodes::Filter { node, f: self.f }
        }
    }
    impl<C: MessageConverter, T, F, R> PipeGraphTemplate<C> for Map<T, F, R>
    where
        T: PipeGraphTemplate,
        F: FnMut(&<T::Data as Message>::Data) -> R,
    {
        type Data = Msg<R>;
        type Node = T::Node;

        fn into_node(self, features: &Features) -> Self::Node {
            todo!()
        }
    }
    impl<T: PipeGraphTemplate, F> PipeGraphTemplate for Fork<T, F> {
        type Data = T::Data;
    }
    impl<T> PipeGraphTemplate for Branch<T> {
        type Data = T;
    }
    impl<T: PipeGraphTemplate> PipeGraphTemplate for Write<T> {
        type Data = T::Data;
    }
}

pub mod nodes {
    use std::{
        future::Future,
        marker::PhantomData,
        pin::Pin,
        task::{Context, Poll},
    };

    use futures_util::{future::Ready, stream, FutureExt, Stream, StreamExt, TryStreamExt};
    use zmaxion_utils::convert::IntoTryFuture;

    use crate::templates::Message;

    pub trait PipeGraphNode {
        type Item: Future<Output = Option<Result<Self::T, Self::E>>>;
        type T: Message;
        type E;
        fn pull(&mut self) -> Self::Item;
    }
    impl<T: PipeGraphNode, F, Fm, Rm> Stream for Filter<T, F, Fm, Rm> {
        type Item = ();

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            todo!()
        }
    }

    pub struct Filter<T, F, Fm, Rm> {
        pub(super) node: T,
        pub(super) f: F,
        _t: PhantomData<(Fm, Rm)>,
    }

    impl<N, F, R, Fm, Rm> PipeGraphNode for Filter<N, F, Fm, Rm>
    where
        N: PipeGraphNode,
        F: FnMut(&Result<N::T, N::E>) -> R,
        R: IntoTryFuture<Fm, Rm, T = bool>,
    {
        type E = N::E;
        type Item = Ready<Option<Result<Self::T, Self::E>>>;
        type T = N::T;

        fn pull(&mut self) -> Self::Item {
            let a = futures_util::stream::repeat_with(|| self.node.pull())
                .then(|x| x)
                .map(|x| stream::iter(x))
                .flatten()
                .then(|x| async { (x, (self.f)(&x).into_try_future().await) })
                .filter(|x| async {
                    match x.1 {
                        Ok(true) => true,
                        _ => false,
                    }
                })
                .map(|x| x.0)
                .next();
            a
            //            panic!()
        }
    }
}

pub mod nodes2 {
    use std::{future::Future, marker::PhantomData};

    use futures_util::{future::Ready, stream, FutureExt, StreamExt, TryStreamExt};
    use zmaxion_utils::convert::IntoTryFuture;

    use crate::templates::{Message, MessageConverter};

    pub trait PipeGraphNode {
        type Converter: MessageConverter;
        type Item: Future<Output = Option<Result<Self::T, Self::E>>>;
        type T: Message;
        type E;
        fn pull(&mut self) -> Self::Item;
    }

    pub struct Filter<N, F, C> {
        pub(super) node: N,
        pub(super) f: F,
        _t: PhantomData<C>,
    }

    impl<N, F, R, C> PipeGraphNode for Filter<N, F, C>
    where
        N: PipeGraphNode,
        F: FnMut(&Result<N::T, N::E>) -> R,
        //            R: IntoTryFuture<Fm, Rm, T = bool>,
    {
        type Converter = C;
        type E = N::E;
        type Item = Ready<Option<Result<Self::T, Self::E>>>;
        type T = N::T;

        fn pull(&mut self) -> Self::Item {
            let a = futures_util::stream::repeat_with(|| self.node.pull())
                .then(|x| x)
                .map(|x| stream::iter(x))
                .flatten()
                .then(|x| async { (x, (self.f)(&x).into_try_future().await) })
                .filter(|x| async {
                    match x.1 {
                        Ok(true) => true,
                        _ => false,
                    }
                })
                .map(|x| x.0)
                .next();
            a
            //            panic!()
        }
    }
}
