use std::{
    iter::{Chain, Flatten, Map},
    marker::PhantomData,
    ops::Range,
    pin::Pin,
    slice::Iter,
    task::{Context, Poll},
};

use futures_lite::stream::StreamExt;
use futures_util::{
    future::{ready, Ready},
    stream, Stream,
};
use zmaxion_core::prelude::*;
use zmaxion_flow::{
    default::DefaultPoolFetcher,
    flow::{EventConsumer, MessageFetch, MessagePool, Poolfetch, ToEventWriter, ToFlower},
    prelude::Flower,
    Event, EventRange, EventsAll,
};
use zmaxion_utils::{
    pool::{PoolArc, PoolItem},
    prelude::*,
};

use crate::bevy_ecs::system::SystemParam;

struct AllEventsIter<'a, Fetcher: Poolfetch<'a, E>, E> {
    events: &'a EventsAll,
    pool: &'a Fetcher::Pool,
    i: usize,
    _t: PhantomData<(Fetcher, E)>,
}
impl<'a, Fetcher: Poolfetch<'a, E>, E> Iterator for AllEventsIter<'a, Fetcher, E> {
    type Item = <Fetcher::Pool as MessageFetch<'a>>::Message;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.pool.len() {
            let value = Some(self.pool.at(self.i as u32, &self.events.data));
            self.i += 1;
            value
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.pool.len(), Some(self.pool.len()))
    }
}

impl<'a, Fetcher: Poolfetch<'a, E>, E> AllEventsIter<'a, Fetcher, E> {
    pub fn new(events: &'a EventsAll) -> Self {
        let pool = Fetcher::fetch(&events.data);
        Self {
            events,
            pool,
            i: 0,
            _t: Default::default(),
        }
    }
}

struct EventRangeIter<'a, Fetcher: Poolfetch<'a, E>, E> {
    events: &'a EventRange,
    pool: &'a Fetcher::Pool,
    i: u32,
    _t: PhantomData<(Fetcher, E)>,
}
impl<'a, Fetcher: Poolfetch<'a, E>, E> Iterator for EventRangeIter<'a, Fetcher, E> {
    type Item = <Fetcher::Pool as MessageFetch<'a>>::Message;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.events.end {
            let value = Some(self.pool.at(self.i, &self.events.data));
            self.i += 1;
            value
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let min = (self.events.end - self.events.start) as usize;
        (min, Some(min))
    }
}

impl<'a, Fetcher: Poolfetch<'a, E>, E> EventRangeIter<'a, Fetcher, E> {
    pub fn new(events: &'a EventRange) -> Self {
        let pool = Fetcher::fetch(&events.data);
        Self {
            events,
            pool,
            i: events.start,
            _t: Default::default(),
        }
    }
}

struct EventIter<'a, Fetcher: Poolfetch<'a, E>, E> {
    event: &'a Event,
    pool: &'a Fetcher::Pool,
    finished: bool,
    _t: PhantomData<(Fetcher, E)>,
}

impl<'a, Fetcher: Poolfetch<'a, E>, E> Iterator for EventIter<'a, Fetcher, E> {
    type Item = <Fetcher::Pool as MessageFetch<'a>>::Message;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            None
        } else {
            let value = Some(self.pool.at(self.event.row as u32, &self.event.data));
            self.finished = true;
            value
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, Some(1))
    }
}

impl<'a, Fetcher: Poolfetch<'a, E>, E> EventIter<'a, Fetcher, E> {
    pub fn new(events: &'a Event) -> Self {
        let pool = Fetcher::fetch(&events.data);
        Self {
            event: events,
            pool,
            finished: false,
            _t: Default::default(),
        }
    }
}

type EventStream<'a, Fetcher, E> = stream::Iter<
    Chain<
        Chain<
            Flatten<Map<Iter<'a, EventsAll>, fn(&'a EventsAll) -> AllEventsIter<'a, Fetcher, E>>>,
            Flatten<
                Map<Iter<'a, EventRange>, fn(&'a EventRange) -> EventRangeIter<'a, Fetcher, E>>,
            >,
        >,
        Flatten<Map<Iter<'a, Event>, fn(&'a Event) -> EventIter<'a, Fetcher, E>>>,
    >,
>;

#[derive(Default)]
struct DfEvents {
    events: Vec<Event>,
    all_events: Vec<EventsAll>,
    event_ranges: Vec<EventRange>,
}

impl DfEvents {
    pub fn stream<'a, Fetcher: Poolfetch<'a, E>, E>(&'a self) -> EventStream<'a, Fetcher, E> {
        fn map_all_events<'a, Fetcher: Poolfetch<'a, E>, E>(
            e: &'a EventsAll,
        ) -> AllEventsIter<'a, Fetcher, E> {
            AllEventsIter::new(e)
        }
        fn map_range_events<'a, Fetcher: Poolfetch<'a, E>, E>(
            e: &'a EventRange,
        ) -> EventRangeIter<'a, Fetcher, E> {
            EventRangeIter::new(e)
        }
        fn map_event<'a, Fetcher: Poolfetch<'a, E>, E>(e: &'a Event) -> EventIter<'a, Fetcher, E> {
            EventIter::new(e)
        }
        stream::iter(
            self.all_events
                .iter()
                .map(
                    map_all_events::<'a, Fetcher, E>
                        as fn(&'a EventsAll) -> AllEventsIter<'a, Fetcher, E>,
                )
                .flatten()
                .chain(
                    self.event_ranges
                        .iter()
                        .map(
                            map_range_events::<'a, Fetcher, E>
                                as fn(&'a EventRange) -> EventRangeIter<'a, Fetcher, E>,
                        )
                        .flatten(),
                )
                .chain(
                    self.events
                        .iter()
                        .map(
                            map_event::<'a, Fetcher, E>
                                as fn(&'a Event) -> EventIter<'a, Fetcher, E>,
                        )
                        .flatten(),
                ),
        )
    }
}

#[derive(Component)]
pub struct BevyTopic<P> {
    read: DfEvents,
    writer: DfEvents,
    _t: PhantomData<P>,
}

impl<P: Resource> BevyTopic<P> {
    pub fn update_system(mut query: Query<&mut BevyTopic<P>>) {
        for mut topic in &mut query.iter_mut() {
            topic.update();
        }
    }

    pub fn update(&mut self) {
        std::mem::swap(&mut self.read, &mut self.writer);
        self.writer.all_events.clear();
        self.writer.event_ranges.clear();
        self.writer.events.clear();
    }
}

#[derive(SystemParam)]
pub struct GenericGlobalBevyReader<'w, 's, P: Resource, E: 'static> {
    query: Query<'w, 's, &'static BevyTopic<P>>,
    global_id: Res<'w, GlobalEntity>,
    #[system_param(ignore)]
    _t: PhantomData<E>,
}

impl<'w: 's, 's, P: Resource, E> GenericGlobalBevyReader<'w, 's, P, E> {
    pub fn flow<'p>(&'p mut self) -> BevyFlower<'p, P, E>
    where
        P: Poolfetch<'p, E>,
        's: 'p,
    {
        self.to_flower().unwrap()
    }
}

pub type GlobalBevyReader<'w, 's, E> = GenericGlobalBevyReader<'w, 's, DefaultPoolFetcher, E>;

impl<'w: 's, 's: 'p, 'p, P: Poolfetch<'p, E>, E> ToFlower<'p>
    for GenericGlobalBevyReader<'w, 's, P, E>
{
    type Flower = BevyFlower<'p, P, E>;

    fn to_flower(&'p mut self) -> AnyResult<Self::Flower> {
        let topic = self.query.get(self.global_id.0).unwrap();
        Ok(BevyFlower {
            stream: topic.read.stream(),
        })
    }
}

pub struct BevyFlower<'s, Pool: Poolfetch<'s, E>, E> {
    stream: EventStream<'s, Pool, E>,
}

impl<'s, E: 'static, Pool> Flower<'s> for BevyFlower<'s, Pool, E>
where
    Pool: Poolfetch<'s, E>,
{
    type Item = <Pool::Pool as MessageFetch<'s>>::Message;

    const IS_ASYNC: bool = false;

    fn poll_begin(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<AnyResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next(ctx)
    }

    fn poll_end(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }
}

#[derive(SystemParam)]
pub struct GenericGlobalBevyWriter<'w, 's, P: Resource, E: 'static> {
    query: Query<'w, 's, &'static BevyTopic<P>>,
    global_id: Res<'w, GlobalEntity>,
    #[system_param(ignore)]
    _t: PhantomData<E>,
}
pub type GlobalBevyWriter<'w, 's, E> = GenericGlobalBevyWriter<'w, 's, DefaultPoolFetcher, E>;

pub struct BevyConsumer<'s> {
    write: &'s mut DfEvents,
}

impl<'s> EventConsumer for BevyConsumer<'s> {
    fn write_one(&mut self, event: Event) {
        self.write.events.push(event);
    }

    fn write_range(&mut self, event: EventRange) {
        self.write.event_ranges.push(event);
    }

    fn write_all(&mut self, event: EventsAll) {
        self.write.all_events.push(event);
    }
}

impl<'w, 's, P: Resource, E> ToEventWriter<P, E> for GenericGlobalBevyWriter<'w, 's, P, E> {
    type Writer = BevyConsumer<'s>;

    fn to_event_writer(&mut self) -> AnyResult<Self::Writer> {
        let topic = self.global_id.get(&self.query);
        let write = unsafe { &mut *(&topic.writer as *const DfEvents as *mut DfEvents) };
        Ok(BevyConsumer { write })
    }
}
