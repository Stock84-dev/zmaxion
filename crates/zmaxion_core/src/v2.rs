use std::marker::PhantomData;

use bevy_ecs::system::Resource;

pub trait TopicProvider {}

pub trait TopicInjector<'w, 's, TopicStrategy, MessageStrategy, T> {
    type Reader;
    type Writer;
}

pub trait QueueInjector<'w, 's, QueueStrategy, MessageStrategy, T> {
    type Reader;
    type Writer;
}

struct SystemContext;
struct PeekableStrategy;
struct SystemStrategy;
struct AsyncRuntime;
struct SerialRuntime;
struct BevyRuntime;

// impl<'w, 's, T: Resource> TopicInjector<'w, 's, (SystemStrategy,), T> for SystemContext {
//    type Reader = SystemReader<'w, 's, T>;
//    type Writer = SystemWriter<'w, 's, T>;
//}

pub struct SystemReader<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

pub struct SystemWriter<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

pub struct AsyncReader<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

pub struct AsyncWriter<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

// impl<'w, 's, T: Resource> TopicInjector<'w, 's, (PeekableStrategy, SystemStrategy,), T> for
// SystemContext {    type Reader = PeekableSystemReader<'w, 's, T>;
//    type Writer = PeekableSystemWriter<'w, 's, T>;
//}

pub struct PeekableSystemReader<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

pub struct PeekableSystemWriter<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

pub struct PeekableAsyncReader<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

pub struct PeekableAsyncWriter<'w, 's, T> {
    _t: PhantomData<&'w ()>,
    _t2: PhantomData<&'s ()>,
    _t3: PhantomData<T>,
}

// pub trait PipelineStrategy {}
// pub trait PipeStrategy {}
// pub trait TopicStrategy {}
// pub trait MessageStrategy {}
//
// pub trait Router {}
//
// struct Context<PipelineStrategy, PipeStrategy, RuntimeStrategy> {
//    _t: PhantomData<(PipelineStrategy, PipeStrategy, RuntimeStrategy)>,
//}

// struct None;

// impl<'w, 's, T: Resource> TopicInjector<'w, 's, None, (), T> for Context<None, None> {
//    type Reader = SystemReader<'w, 's, T>;
//    type Writer = SystemWriter<'w, 's, T>;
//}
// impl<'w, 's, T: Resource> TopicInjector<'w, 's, SystemStrategy, (), T> for Context<None, None> {
//    type Reader = SystemReader<'w, 's, T>;
//    type Writer = SystemWriter<'w, 's, T>;
//}
// pub trait IsPeekable {}
// impl IsPeekable for (Peekable,) & (F0, Peekable,) & (Peekable, F0,)

// impl<'w, 's, T: Resource, M> TopicInjector<'w, 's, SystemStrategy, M, T> for Context<None, None>
// where
//    M: IsPeekable,
//{
//    type Reader = PeekableSystemReader<'w, 's, T>;
//    type Writer = PeekableSystemWriter<'w, 's, T>;
//}
// impl<'w, 's, T: Resource> TopicInjector<'w, 's, SystemStrategy, (PeekableStrategy,), T>
//    for Context<PeekableStrategy, None>
//{
//    type Reader = PeekableSystemReader<'w, 's, T>;
//    type Writer = PeekableSystemWriter<'w, 's, T>;
//}
// pub trait IsSystem {}
// pub trait IsTraceable {}
// pub trait IsAsync {}
// pub trait IsGrouped {}
// pub trait OnBevyRuntime {}
// pub trait OnAsyncRuntime {}
//
// pub struct Async;
// struct TracedMessage<T> {
//    _t: PhantomData<T>,
//}
// struct Traced;
// struct System;
// struct Traceable;
// struct NotTraceable;
//
// impl !IsAsync for System {
//}
// impl<'w, 's, L, P, T, M, E: Resource> TopicInjector<'w, 's, T, M, E> for Context<L, P, System> {
//    type Reader = SystemReader<'w, 's, E>;
//    type Writer = SystemWriter<'w, 's, E>;
//}

// impl<'w, 's, L, P, E: Resource> TopicInjector<'w, 's, (Async, ()), ((), ()), E>
//    for Context<L, P, System>
//{
//    type Reader = AsyncReader<'w, 's, E>;
//    type Writer = AsyncWriter<'w, 's, E>;
//}
// impl<'w, 's, L, P, E: Resource> TopicInjector<'w, 's, (System, ()), (Traceable, ()), E>
//    for Context<L, P, System>
//{
//    type Reader = SystemReader<'w, 's, TracedMessage<E>>;
//    type Writer = SystemWriter<'w, 's, TracedMessage<E>>;
//}

// impl<'w, 's, L, P, T, M, E: Resource> TopicInjector<'w, 's, T, M, E> for Context<L, P, System>
// where
//    T: IsAsync,
//{
//    type Reader = AsyncReader<'w, 's, E>;
//    type Writer = AsyncWriter<'w, 's, E>;
//}

// struct Ctx<T> {
//    _t: PhantomData<T>,
//}

// impl<T> Ctx<T> {
//    fn into_pipe_factory(self)
//}

// trait Template {
//    type Next;
//}

// pub trait IntoTemplate {
//    fn into_template(self) -> Box<dyn Template>;
//}

use enumflags2::{bitflags, BitFlag, BitFlags};

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
enum PipelineReq {
    Empty,
}
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
enum RuntimeReq {
    Serial,
    Bevy,
    Async,
}
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
enum PipeReq {
    Empty,
}
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
enum TopicParamReq {
    System,
    Async,
}
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
enum TopicDirectionReq {
    Reader,
    Writer,
}
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
enum QueueParamReq {
    Empty,
}
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
enum MessageReq {
    Peekable,
    Traced,
}

struct TopicReq {
    topic: BitFlags<TopicParamReq>,
    direction: BitFlags<TopicDirectionReq>,
    message: BitFlags<MessageReq>,
}

enum ParamReq {
    Topic(TopicReq),
    Queue(BitFlags<QueueParamReq>, BitFlags<MessageReq>),
}

struct StaticReq {
    runtime: BitFlags<RuntimeReq>,
    pipeline: BitFlags<PipelineReq>,
    pipe: BitFlags<PipelineReq>,
    param: Vec<Option<ParamReq>>,
}

trait PipelineNeeds {}

struct Req<L, P, A, M> {
    _t: PhantomData<(L, P, A, M)>,
}

struct PeekableReq;
trait Peekable {
    type M;
    fn peek(&self) -> &Self::M;
}

fn process2_static_req() -> StaticReq {
    StaticReq {
        runtime: RuntimeReq::empty(),
        pipeline: PipelineReq::empty(),
        pipe: PipelineReq::empty(),
        param: vec![Some(ParamReq::Topic(TopicReq {
            topic: TopicParamReq::empty(),
            direction: TopicDirectionReq::Reader.into(),
            message: MessageReq::Peekable.into(),
        }))],
    }
}

trait IsPeekable {}

struct Complex<A, B> {
    _t: PhantomData<(A, B)>,
}

impl<T> IsPeekable for Complex<(PeekableReq,), T> {
}

impl IsPeekable for Box<dyn IsPeekable> {
}

// impl<'w, 's, E, C> TopicInjector<'w, 's, (PeekableReq,), (), E> for C {
//    type Reader = ();
//    type Writer = ();
//}

fn process2<'w, 's, C, Ca, Cb, Cc>(
    topic_a: <Ca as TopicInjector<'w, 's, (), (), i32>>::Reader,
    topic_b: <Cb as TopicInjector<'w, 's, (PeekableReq,), (), String>>::Writer,
    queue_a: <Cc as QueueInjector<'w, 's, (), (), i32>>::Reader,
    mut context: C,
) where
    Ca: TopicInjector<'w, 's, (), (), i32>,
    Cb: TopicInjector<'w, 's, (PeekableReq,), (), String>,
    <Cb as TopicInjector<'w, 's, (PeekableReq,), (), String>>::Writer: Peekable,
    Cc: QueueInjector<'w, 's, (), (), i32>,
{
    topic_b.peek();
}
struct Injector;
struct GenericTopicInjector<T, M, E> {
    _t: PhantomData<(T, M, E)>,
}

impl<'w, 's, E> TopicInjector<'w, 's, (), (), E> for GenericTopicInjector<(), (), E> {
    type Reader = ();
    type Writer = ();
}
impl<'w, 's, E> TopicInjector<'w, 's, (), (), E> for Injector {
    type Reader = ();
    type Writer = ();
}

impl<'w, 's, E> TopicInjector<'w, 's, (PeekableReq,), (), E>
    for GenericTopicInjector<(PeekableReq,), (), E>
{
    type Reader = ();
    type Writer = ();
}
impl<'w, 's, E> TopicInjector<'w, 's, (PeekableReq,), (), E> for Injector {
    type Reader = ();
    type Writer = ();
}

impl Peekable for () {
    type M = ();

    fn peek(&self) -> &Self::M {
        todo!()
    }
}

trait PeekableMarker {}
trait SystemMarker {}
// async 2
// peekable 2
// group 2
// ackable 2
// traceable 2
// params 16

struct System;

impl<'w, 's, E> TopicInjector<'w, 's, (PeekableReq, System), (), E>
    for GenericTopicInjector<(), (), E>
{
    type Reader = ();
    type Writer = ();
}
impl<'w, 's, E> TopicInjector<'w, 's, (PeekableReq, System), (), E> for Injector {
    type Reader = ();
    type Writer = ();
}
impl<'w, 's, E> QueueInjector<'w, 's, (), (), E> for Injector {
    type Reader = ();
    type Writer = ();
}

// fn a() {
//    if false {
//        let ptr = process2::<
//            (),
//            GenericTopicInjector<(), (), ()>,
//            Injector,
//            Injector,
//            //            GenericTopicInjector<(), (), i32>,
//            //            GenericTopicInjector<(PeekableReq,), (), String>,
//            //            GenericTopicInjector<(PeekableReq,), (), i32>,
//        >;
//    }
//}

trait TopicWalker {
    type Next;
    const REQ: StaticReq;
}

struct TopicTypes<Trait, Injected> {
    _t: PhantomData<(Trait, Injected)>,
}

fn big<A, B, C, D>() {
}

// macro_rules! topic_walker {
//    ($f:ident, $($t:ident),+) => {
//        if req.topic.contains(TopicParamReq::System) {
//            if req.direction.contains(TopicDirectionReq::Reader) {
//                if req.message.contains(MessageReq::Peekable) {
//                    if req.message.contains(MessageReq::Traced) {
//                        type Param0 = SystemTopicReader<Acked<Traced<T>>>;
//                        Box::new($f::<$($t,)+
//                    } else {
//                        Box::new($f::<$($t,)+ SystemTopicReader<Acked<T>>>)
//                    }
//                } else {
//                    if req.message.contains(MessageReq::Traced) {
//                        Box::new($f::<$($t,)+ SystemTopicReader<Traced<T>>>)
//                    } else {
//                        Box::new($f::<$($t,)+ SystemTopicReader<T>>)
//                    }
//                }
//            } else {
//                if req.message.contains(MessageReq::Peekable) {
//                    if req.message.contains(MessageReq::Traced) {
//                        Box::new($f::<$($t,)+ SystemTopicReader<Acked<Traced<T>>>>)
//                    } else {
//                        Box::new($f::<$($t,)+ SystemTopicReader<Acked<T>>>)
//                    }
//                } else {
//                    if req.message.contains(MessageReq::Traced) {
//                        Box::new($f::<$($t,)+ SystemTopicReader<Traced<T>>>)
//                    } else {
//                        Box::new($f::<$($t,)+ SystemTopicReader<T>>)
//                    }
//                }
//            }
//        } else {
//            if req.topic.contains(TopicParamReq::Async) {
//                if req.direction.contains(TopicDirectionReq::Reader) {
//                    if req.message.contains(MessageReq::Peekable) {
//                        if req.message.contains(MessageReq::Traced) {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<Acked<Traced<T>>>>)
//                        } else {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<Acked<T>>>)
//                        }
//                    } else {
//                        if req.message.contains(MessageReq::Traced) {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<Traced<T>>>)
//                        } else {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<T>>)
//                        }
//                    }
//                } else {
//                    if req.message.contains(MessageReq::Peekable) {
//                        if req.message.contains(MessageReq::Traced) {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<Acked<Traced<T>>>>)
//                        } else {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<Acked<T>>>)
//                        }
//                    } else {
//                        if req.message.contains(MessageReq::Traced) {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<Traced<T>>>)
//                        } else {
//                            Box::new($f::<$($t,)+ AsyncTopicReader<T>>)
//                        }
//                    }
//                }
//            }
//        }
//    };
//}

// fn test() {
//    let a = topic_walker!(big, i8, i16, i32);
//}

// macro_rules! inject_runtime {
//    ($req:expr, $block:block) => {
//        if $req.runtime.contains(RuntimeReq::Bevy) {
//            type Runtime = BevyRuntime;
//            $block
//        } else if $req.runtime.contains(RuntimeReq::Serial) {
//            type Runtime = SerialRuntime;
//            $block
//        } else {
//            type Runtime = AsyncRuntime;
//            $block
//        }
//    };
//}

// macro_rules! inject_params {
//    ($($input:ident),*) => {};
//    ($selected:ident; $($out:ident),*) => {};
//}

struct Acked<T>(T);
struct Traced<T>(T);

//#[rustfmt::skip]
// fn a<T>(req: StaticReq) {
//    inject_runtime!(req, {});
//    if let Some(param) = req.param.get(0) {
//        if let Some(param) = param {
//            type Param0;
//            if let Some(param) = req.param.get(1) {
//                if let Some(param) = param {
//                    type Param1;
//                } else {
//                    //
//                }
//            } else {
//                //
//            }
//        } else {
//            //
//        }
//    } else {
//        //
//    }
//    if req.runtime.contains(RuntimeReq::Bevy) {
//        type Runtime = BevyRuntime;
//    } else if req.runtime.contains(RuntimeReq::Serial) {
//        type Runtime = SerialRuntime;
//    } else {
//        type Runtime = AsyncRuntime;
//    }
//    if req.topic.contains(TopicParamReq::System) {
//        if req.direction.contains(TopicDirectionReq::Reader) {
//            if req.message.contains(MessageReq::Peekable) {
//                if req.message.contains(MessageReq::Traced) {
//                    SystemTopicReader(Vec::<Acked<Traced<T>>>::new())
//                } else {
//                    SystemTopicReader(Vec::<Acked<T>>::new())
//                }
//            } else {
//                if req.message.contains(MessageReq::Traced) {
//                    SystemTopicReader(Vec::<Traced<T>>::new())
//                } else {
//                    SystemTopicReader(Vec::<T>::new())
//                }
//            }
//        } else {
//            if req.message.contains(MessageReq::Peekable) {
//                if req.message.contains(MessageReq::Traced) {
//                    SystemTopicWriter(Vec::<Acked<Traced<T>>>::new())
//                } else {
//                    SystemTopicWriter(Vec::<Acked<T>>::new())
//                }
//            } else {
//                if req.message.contains(MessageReq::Traced) {
//                    SystemTopicWriter(Vec::<Traced<T>>::new())
//                } else {
//                    SystemTopicWriter(Vec::<T>::new())
//                }
//            }
//        }
//    } else {
//        if req.direction.contains(TopicDirectionReq::Reader) {
//            if req.message.contains(MessageReq::Peekable) {
//                if req.message.contains(MessageReq::Traced) {
//                    AsyncTopicReader(Vec::<Acked<Traced<T>>>::new())
//                } else {
//                    AsyncTopicReader(Vec::<Acked<T>>::new())
//                }
//            } else {
//                if req.message.contains(MessageReq::Traced) {
//                    AsyncTopicReader(Vec::<Traced<T>>::new())
//                } else {
//                    AsyncTopicReader(Vec::<T>::new())
//                }
//            }
//        } else {
//            if req.message.contains(MessageReq::Peekable) {
//                if req.message.contains(MessageReq::Traced) {
//                    AsyncTopicWriter(Vec::<Acked<Traced<T>>>::new())
//                } else {
//                    AsyncTopicWriter(Vec::<Acked<T>>::new())
//                }
//            } else {
//                if req.message.contains(MessageReq::Traced) {
//                    AsyncTopicWriter(Vec::<Traced<T>>::new())
//                } else {
//                    AsyncTopicWriter(Vec::<T>::new())
//                }
//            }
//        }
//    }
//}

trait Topic {}

struct SystemTopicReader<T>(Vec<T>);
struct SystemTopicWriter<T>(Vec<T>);
struct AsyncTopicReader<T>(Vec<T>);
struct AsyncTopicWriter<T>(Vec<T>);
impl<T> Topic for SystemTopicReader<T> {
}
impl<T> Topic for SystemTopicWriter<T> {
}

type Walker = (
    TopicTypes<((), ()), ((), ())>,
    TopicTypes<((), ()), ((), ())>,
    TopicTypes<((), ()), ((), ())>,
    TopicTypes<((), ()), ((), ())>,
    TopicTypes<((), ()), ((), ())>,
);

// if let Some(param) = req.param.get(2) {
// if let Some(param) = param {
// type Param2;
// if let Some(param) = req.param.get(3) {
// if let Some(param) = param {
// type Param3;
// if let Some(param) = req.param.get(4) {
// if let Some(param) = param {
// type Param4;
// if let Some(param) = req.param.get(5) {
// if let Some(param) = param {
// type Param5;
// if let Some(param) = req.param.get(6) {
// if let Some(param) = param {
// type Param6;
// if let Some(param) = req.param.get(7) {
// if let Some(param) = param {
// type Param7;
// if let Some(param) = req.param.get(8) {
// if let Some(param) = param {
// type Param8;
// if let Some(param) = req.param.get(9) {
// if let Some(param) = param {
// type Param9;
// if let Some(param) = req.param.get(10) {
// if let Some(param) = param {
// type Param10;
// if let Some(param) = req.param.get(11) {
// if let Some(param) = param {
// type Param11;
// if let Some(param) = req.param.get(12) {
// if let Some(param) = param {
// type Param12;
// if let Some(param) = req.param.get(13) {
// if let Some(param) = param {
// type Param13;
// if let Some(param) = req.param.get(14) {
// if let Some(param) = param {
// type Param14;
// if let Some(param) = req.param.get(15) {
// if let Some(param) = param {
// type Param15;
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }
// } else {
//
// }

struct Param;

//#[zmaxion_derive::process]
// fn process(p0: Param) {
//}
