use std::{
    future::Future,
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};

use ergnomics::prelude::*;
use futures_util::{pin_mut, ready};
use pin_project_lite::pin_project;
use zmaxion_rt::prelude::*;
use zmaxion_utils::{
    pool::{ArcPoolItem, PoolArc, PoolArcMut, PoolItem},
    prelude::AnyResult,
};

use crate::{
    flow::{
        EventConsumer, MapableMessage, Message, PoolBuilder, Pooled, PooledMessage,
        ToAsyncEventWriter, ToEventWriter,
    },
    Event, EventRange, EventsAll,
};

pub trait Flower<'a> {
    const IS_ASYNC: bool;
    //    type Locked;
    //    type Future: Future<Output = AnyResult<()>>;
    //    type EndFuture: Future<Output = AnyResult<()>>;
    type Item: Message<'a>;
    //    fn lock(self) -> Self::Locked;
    fn poll_begin(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<AnyResult<()>>;
    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
    fn map<F, O, E>(self, f: F) -> Map<Self, F, E>
    where
        F: FnMut(<Self::Item as Message<'a>>::Value) -> O,
        Self: Sized,
    {
        Map {
            inner: self,
            f,
            _t: Default::default(),
        }
    }
    fn write<W>(self, mut writer: W) -> Write<'a, Self, W, <Self::Item as Message<'a>>::Pool>
    where
        Self: Sized,
        W: ToAsyncEventWriter<
                <<Self as Flower<'a>>::Item as Message<'a>>::Pool,
                <<Self::Item as Message<'a>>::PooledMessage as Message<'a>>::Value,
            > + ToEventWriter<
                <<Self as Flower<'a>>::Item as Message<'a>>::Pool,
                <<<Self as Flower<'a>>::Item as Message<'a>>::PooledMessage as Message<'a>>::Value,
            >,
    {
        Write {
            inner: self,
            writer_builder: Some(writer.to_async_event_writer()),
            writer: None,
            start: 0,
            current: 0,
            ended: false,
            polled_inner_in_begin: false,
            pool: <<Self::Item as Message<'a>>::Pool as PoolBuilder<Self::Item>>::create(),
            ref_pool: None,
        }
    }
    fn poll_end(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<()>;

    /// Blocks the current thread until all messages in a batch have been processed. Prefer using
    /// this method instead of runtime.block_on(...) because this function is optimized if a future
    /// is always ready. As of rustc 1.64 futures that are immediately ready don't get compiled
    /// away even when lto is on. Noop waker needs to be used instead.
    fn run(mut self) -> AnyResult<()>
    where
        Self: Sized,
    {
        let mut fut = self.run_async();
        pin_mut!(fut);
        (&mut fut).block()?;
        if Self::IS_ASYNC {
            (&mut fut).block()?;
        } else {
            poll_and_finish(&mut fut)?;
        };
        (&mut fut).block()?;
        Ok(())
    }

    fn run_async(self) -> FlowFuture<Self>
    where
        Self: Sized,
    {
        FlowFuture {
            fut: self.run_async_raw(),
            yield_count: 0,
        }
    }

    fn run_async_raw(self) -> RawFlowFuture<Self>
    where
        Self: Sized,
    {
        RawFlowFuture {
            flower: self,
            sm: FlowState::Begin,
        }
    }
}

#[derive(Clone, Copy)]
pub enum FlowState {
    Begin,
    Stream,
    End,
}

pin_project! {
    /// This future must be pulled until it returns a total of 3 Poll::Ready
    pub struct RawFlowFuture<F> {
        #[pin]
        flower: F,
        sm: FlowState,
    }
}

impl<'a, F: Flower<'a>> Future for RawFlowFuture<F> {
    type Output = AnyResult<FlowState>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        match *this.sm {
            FlowState::Begin => {
                ready!(this.flower.poll_begin(cx))?;
                *this.sm = FlowState::Stream;
                return Poll::Ready(Ok(FlowState::Begin));
            }
            FlowState::Stream => {
                if ready!(this.flower.poll_next(cx)).is_none() {
                    *this.sm = FlowState::End;
                    return Poll::Ready(Ok(FlowState::Stream));
                }
            }
            FlowState::End => {
                ready!(this.flower.poll_end(cx));
                return Poll::Ready(Ok(FlowState::End));
            }
        }
        Poll::Pending
    }
}

pin_project! {
    pub struct FlowFuture<F> {
        #[pin]
        fut: RawFlowFuture<F>,
        yield_count: u8
    }
}

impl<'a, T: Flower<'a>> Future for FlowFuture<T> {
    type Output = AnyResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        while *this.yield_count < 3 {
            ready!(this.fut.as_mut().poll(cx))?;
            *this.yield_count += 1;
        }
        Poll::Ready(Ok(()))
    }
}

// pin_project doesn't support trait composition, creating a supertrait instead
pub trait ToEventWriterCombo<'a, I: Flower<'a>, P>:
    ToAsyncEventWriter<P, <<I::Item as Message<'a>>::PooledMessage as Message<'a>>::Value>
    + ToEventWriter<
        <<I as Flower<'a>>::Item as Message<'a>>::Pool,
        <<<I as Flower<'a>>::Item as Message<'a>>::PooledMessage as Message<'a>>::Value,
    >
{
}

impl<'a, T, I, P> ToEventWriterCombo<'a, I, P> for T
where
    I: Flower<'a>,
    T: ToAsyncEventWriter<P, <<I::Item as Message<'a>>::PooledMessage as Message<'a>>::Value>
        + ToEventWriter<
            <<I as Flower<'a>>::Item as Message<'a>>::Pool,
            <<<I as Flower<'a>>::Item as Message<'a>>::PooledMessage as Message<'a>>::Value,
        >,
{
}

pin_project! {
    #[must_use = "Flowers do nothing unless polled"]
    pub struct Write<'a,
        I: Flower<'a>,
        W: ToEventWriterCombo<'a, I, P>,
        P: PoolItem,
    > {
        #[pin]
        inner: I,
        #[pin]
        writer_builder: Option<W::Future>,
        writer: Option<<W as ToAsyncEventWriter<P,<<I::Item as Message<'a>>::PooledMessage as Message<'a>>::Value>>::Writer>,
        start: u32,
        current: u32,
        ended: bool,
        polled_inner_in_begin: bool,
        pool: Option<PoolArcMut<P>>,
        ref_pool: Option<PoolArc<dyn PoolItem>>,
    }
}

fn flush<'a, I, W, P>(
    start: &mut u32,
    current: &mut u32,
    writer: &mut Option<
        <W as ToAsyncEventWriter<
            P,
            <<I::Item as Message<'a>>::PooledMessage as Message<'a>>::Value,
        >>::Writer,
    >,
    ref_pool: &PoolArc<dyn PoolItem>,
    id: u32,
) where
    I: Flower<'a>,
    W: ToEventWriterCombo<'a, I, P>,
    P: PoolItem,
{
    if *current - *start == 0 {
        writer.some_mut(|x| {
            x.write_one(Event {
                data: ref_pool.clone(),
                row: *current,
            })
        });
    } else {
        writer.some_mut(|x| {
            x.write_range(EventRange {
                data: ref_pool.clone(),
                start: *start,
                end: *current + 1,
            })
        });
    }
    *current = id;
    *start = id;
}

impl<'a, I, W, P> Flower<'a> for Write<'a, I, W, P>
where
    I: Flower<'a>,
    P: PoolItem + PoolBuilder<I::Item>,
    W: ToEventWriterCombo<'a, I, P>,
    I::Item: Message<'a, Pool = P>,
{
    type Item = <I::Item as Message<'a>>::PooledMessage;

    const IS_ASYNC: bool = I::IS_ASYNC;

    fn poll_begin(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<AnyResult<()>> {
        let mut this = self.project();
        loop {
            if *this.polled_inner_in_begin {
                match this.writer_builder.as_mut().as_pin_mut() {
                    None => return Poll::Ready(Ok(())),
                    Some(builder) => {
                        let writer = ready!(builder.poll(ctx))?;
                        *this.writer = Some(writer);
                        return Poll::Ready(Ok(()));
                    }
                }
            } else {
                ready!(this.inner.as_mut().poll_begin(ctx))?;
                *this.polled_inner_in_begin = true;
            }
        }
    }

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        match ready!(this.inner.poll_next(ctx)) {
            None => Poll::Ready(None),
            Some(value) => {
                match value.dyn_pool() {
                    None => {
                        // owned message will be handled with into_pooled
                    }
                    // message references a pool
                    Some(pool) => {
                        match &mut this.ref_pool {
                            None => {
                                // we don't have any messages stored at the begining
                                *this.ref_pool = Some(pool.clone());
                                let id = value.id();
                                *this.start = id;
                                *this.current = id;
                            }
                            Some(ref_pool) => {
                                let a = &**ref_pool as *const _;
                                let b = &**pool as *const _;
                                if a == b {
                                    let id = value.id();
                                    if *this.current + 1 == id {
                                        *this.current += 1;
                                    } else {
                                        // we have a gap in messages
                                        flush::<I, W, _>(
                                            &mut this.start,
                                            &mut this.current,
                                            &mut this.writer,
                                            ref_pool,
                                            id,
                                        );
                                    }
                                } else {
                                    // pools have changed
                                    let id = value.id();
                                    flush::<I, W, _>(
                                        &mut this.start,
                                        &mut this.current,
                                        &mut this.writer,
                                        ref_pool,
                                        id,
                                    );
                                    let pool: PoolArc<_> = pool.clone();
                                    *ref_pool = pool;
                                }
                            }
                        }
                    }
                }
                let pooled;
                unsafe {
                    let pool: &mut Option<PoolArcMut<P>> = std::mem::transmute(&mut this.pool);
                    let ref_pool: &Option<PoolArc<dyn PoolItem>> =
                        std::mem::transmute(&this.ref_pool);

                    pooled = value.into_pooled(pool, ref_pool);
                }
                Poll::Ready(Some(pooled))
            }
        }
    }

    fn poll_end(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<()> {
        let this = self.project();
        ready!(this.inner.poll_end(ctx));
        if I::Item::IS_OWNED && !*this.ended {
            this.pool.some_ref(|pool| {
                if pool.len() != 0 {
                    this.writer.some_mut(|x| {
                        x.write_all(EventsAll {
                            data: unsafe { pool.to_dyn_shared() },
                        })
                    });
                }
            });
            *this.ended = true;
        }
        Poll::Ready(())
    }
}

pin_project! {
    #[must_use = "Flowers do nothing unless polled"]
    pub struct Map<P, F, O> {
        #[pin]
        inner: P,
        f: F,
        _t: PhantomData<O>,
    }
}

impl<'a, P, F, O> Flower<'a> for Map<P, F, O>
where
    F: FnMut(<P::Item as Message<'a>>::Value) -> O,
    P: Flower<'a>,
    P::Item: MapableMessage<'a, <P::Item as Message<'a>>::Value, O>,
{
    type Item = <P::Item as MapableMessage<'a, <P::Item as Message<'a>>::Value, O>>::Mapped;

    const IS_ASYNC: bool = P::IS_ASYNC;

    fn poll_begin(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<AnyResult<()>> {
        let this = self.project();
        this.inner.poll_begin(ctx)
    }

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        Poll::Ready(ready!(this.inner.poll_next(ctx)).map(|x| x.map(&mut this.f)))
    }

    fn poll_end(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<()> {
        let this = self.project();
        this.inner.poll_end(ctx)
    }
}

fn poll_and_finish<F: Future>(fut: F) -> F::Output {
    pin_mut!(fut);
    match fut.poll(&mut Context::from_waker(
        futures_util::task::noop_waker_ref(),
    )) {
        Poll::Ready(output) => output,
        Poll::Pending => {
            panic!("Tried to pool a future that is expected to be ready immediately")
        }
    }
}
