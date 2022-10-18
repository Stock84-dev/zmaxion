use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};

use zmaxion_core::prelude::HashMap;
use zmaxion_utils::prelude::{Mutex, OptionExt, TypeIdExt, TypeName};

struct Series {
    name: String,
    data: Arc<dyn Any>,
}

struct DataFrameTopic {
    // a list of all asynctopic for different dataframe messages
    series: Vec<Series>,
}

struct TopicAccountData {
    balances: VecSeries<f32>,
    n_trades: VecSeries<u32>,
    //    trace_data: VecSeries<()>,
    //    ack_data: VecSeries<()>,
}

pub struct Topic {
    names: Vec<String>,
    read: Vec<Series>,
    write: Vec<Series>,
}

pub struct VecSeries<T> {
    data: Vec<T>,
}

pub trait SeriesTrait {}

// topic contains a message
// a producer allocates dataframe on each epoch, fills it and sends a reference to a topic
//

// backtest accounts topic -> balance: f32
// seek accounts topic -> balance: f32
// avg balance < f32

// fn default_impl<T>() {
//        static COUNTER: (AtomicUsize, Vec<T>) = (AtomicUsize::new(0), Vec::new());
//        println!("default_impl {}: counter was {}", std::any::type_name::<T>(),
// COUNTER.fetch_add(1, Ordering::Relaxed));    }
// default_impl::<i32>();
// default_impl::<i32>();
// default_impl::<i32>();

// pub struct Row {
//    pub data: PoolArc<dyn PoolItem>,
//    pub row: u32,
//}
// pub struct Column {
//    pub data: PoolArc<dyn PoolItem>,
//}
// pub struct ColumnRange {
//    pub data: PoolArc<dyn PoolItem>,
//    pub start: u32,
//    pub len: u32,
//}

// we wrap those messages with tracing data like TracedMessage<Column> AckMessage<Row>
// we recompile a pipe when message format changes

struct Topic2<M> {
    msgs: Vec<M>,
}

struct TracedMessage<M> {
    msg: M,
    trace_data: (),
}

pub trait MessageWrapper<M> {
    type Message;
    fn wrap(&mut self, m: M) -> Self::Message;
}

struct TracedContext;

impl<M> MessageWrapper<M> for TracedContext {
    type Message = TracedMessage<M>;

    fn wrap(&mut self, m: M) -> Self::Message {
        TracedMessage {
            msg: m,
            trace_data: (),
        }
    }
}

pub trait Context {
    fn modify<M>(&mut self, message: &mut <Self as MessageWrapper<M>>::Message)
    where
        Self: MessageWrapper<M>;
}

trait TopicTrait<'w, 's, T, M> {
    type Topic;
}

impl<'w, 's, T, M> TopicTrait<'w, 's, T, M> for TracedContext {
    type Topic = Topic2<TracedMessage<M>>;
}

struct Queue<M> {
    data: Vec<M>,
}

pub trait QueueTrait<'w, 's, M> {
    type Queue;
}
struct Peekable;

type CQueue<'w, 's, C, M> = <C as QueueTrait<'w, 's, M>>::Queue;
type CTopic<'w, 's, C, T, M> = <C as TopicTrait<'w, 's, T, M>>::Topic;
fn process<'w, 's, C>(
    topic_a: CTopic<'w, 's, C, (), i32>,
    topic_b: CTopic<'w, 's, C, Peekable, String>,
    queue_a: CQueue<'w, 's, C, i32>,
    mut context: C,
) where
    C: TopicTrait<'w, 's, (), i32>,
    C: TopicTrait<'w, 's, Peekable, String>,
    C: QueueTrait<'w, 's, i32>,
{
}

fn process2<'w, 's, C, Ca, Cb, Cc>(
    topic_a: <Ca as TopicTrait<'w, 's, (), i32>>::Topic,
    topic_b: <Cb as TopicTrait<'w, 's, Peekable, String>>::Topic,
    queue_a: <Cc as QueueTrait<'w, 's, i32>>::Queue,
    mut context: C,
) where
    Ca: TopicTrait<'w, 's, (), i32>,
    Cb: TopicTrait<'w, 's, Peekable, String>,
    Cc: QueueTrait<'w, 's, i32>,
{
}

//#[process]
// fn process3(
//    topic_a: Topic<i32>,
//    topic_b: Topic<String, (Peekable, Ackable, Traceable)>,
//    queue_a: Queue<i32>,
//) {
//}
// const PIPE_FEATURES: () = ();
//
//#[process(features = PIPE_FEATURES)]
// fn process3(topic: Topic<i32>) {
//    let message_value = 0i32;
//}

// we return things in a tuple, then impl for every element in a tuple se we can add one method that
// adds a factory into hashmap

pub trait MaybePeekable {
    const IS_PEEKABLE: bool;
}

impl MaybePeekable for Peekable {
    const IS_PEEKABLE: bool = true;
}

impl<T> MaybePeekable for T {
    const IS_PEEKABLE: bool = false;
}

struct If<const B: bool>;
pub trait True {}
impl True for If<true> {
}

trait Marker {
    fn call();
}

fn pls<T, const VAL: bool>() {
    if VAL {
        T::call()
    }
}
