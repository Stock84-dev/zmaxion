use std::{cell::RefCell, fmt::Debug, marker::PhantomData, rc::Rc, sync::Arc};

use zmaxion_utils::prelude::Mutex;

// struct Push<T>(PhantomData<T>);
//
// impl<T> Pusher for Push<T> {
//    type Inner = Self;
//    type PushItem = T;
//
//    fn push(&mut self, item: Self::PushItem) {
//        ()
//    }
//
//    fn into_inner(self) -> Self::Inner {
//        self
//    }
//}

struct Filter<P, F> {
    pusher: P,
    f: F,
}

impl<P, F> Pusher for Filter<P, F>
where
    P: Pusher,
    F: FnMut(&P::PullItem) -> bool,
{
    type Inner = P;
    type PullItem = P::PullItem;

    fn pull(&mut self) -> Option<Self::PullItem> {
        while let Some(item) = self.pusher.pull() {
            if (self.f)(&item) {
                return Some(item);
            }
        }
        None
    }
}

struct Map<P, F, T> {
    pusher: P,
    f: F,
    _t: PhantomData<T>,
}

impl<P, F, T> Pusher for Map<P, F, T>
where
    P: Pusher,
    F: FnMut(P::PullItem) -> T,
    T: Debug,
{
    type Inner = P;
    type PullItem = T;

    fn pull(&mut self) -> Option<Self::PullItem> {
        self.pusher.pull().map(|x| (self.f)(x))
    }
}

struct Write<'a, P, W> {
    p: P,
    writer: &'a mut W,
}

impl<'a, P: Pusher, W> Pusher for Write<'a, P, W>
where
    W: Extend<P::PullItem>,
{
    type Inner = P;
    type PullItem = ();

    fn pull(&mut self) -> Option<Self::PullItem> {
        self.p.pull().map(|x| {
            self.writer.extend([x]);
        })
    }
}

struct PusherPortal<T> {
    source: Rc<RefCell<Option<T>>>,
}

impl<T: Debug> Pusher for PusherPortal<T> {
    type Inner = Self;
    type PullItem = T;

    fn pull(&mut self) -> Option<Self::PullItem> {
        let item = { self.source.borrow_mut().take() };
        item
    }
}

struct Fork<A: Pusher, B> {
    pusher: A,
    source: PusherPortal<A::PullItem>,
    forked: B,
}

impl<A, B> Pusher for Fork<A, B>
where
    A: Pusher,
    A::PullItem: Clone,
    B: Pusher,
{
    type Inner = (A::Inner, B::Inner);
    type PullItem = A::PullItem;

    fn pull(&mut self) -> Option<Self::PullItem> {
        let item = self.pusher.pull()?;
        *self.source.source.borrow_mut() = Some(item.clone());
        self.forked.pull();
        Some(item)
    }
}

pub trait Finalize<T> {
    type Output;
    fn from(other: T) -> Self::Output;
}

trait Pusher: Sized {
    type Inner;
    type PullItem: Debug;
    fn pull(&mut self) -> Option<Self::PullItem>;
    fn filter<F: FnMut(&Self::PullItem) -> bool>(self, f: F) -> Filter<Self, F> {
        Filter { pusher: self, f }
    }
    fn map<F: FnMut(Self::PullItem) -> T, T>(self, f: F) -> Map<Self, F, T> {
        Map {
            pusher: self,
            f,
            _t: Default::default(),
        }
    }
    fn fork<F: FnOnce(PusherPortal<Self::PullItem>) -> P, P: Pusher>(self, f: F) -> Fork<Self, P> {
        let portal = PusherPortal {
            source: Rc::new(RefCell::new(None)),
        };
        let pusher = f(PusherPortal {
            source: portal.source.clone(),
        });
        Fork {
            pusher: self,
            source: portal,
            forked: pusher,
        }
    }
    fn write<W: Extend<Self::PullItem>>(self, writer: &mut W) -> Write<Self, W> {
        Write { writer, p: self }
    }
    fn finish<T: Extend<Self::PullItem>>(mut self, writer: &mut T) {
        while let Some(item) = self.pull() {
            writer.extend([item]);
        }
    }
}

pub struct DebugPusher<P> {
    p: P,
}

impl<P> Pusher for DebugPusher<P>
where
    P: Pusher + Finalize<Self>,
    P::PullItem: Debug,
{
    type Inner = P;
    type PullItem = P::PullItem;

    fn pull(&mut self) -> Option<Self::PullItem> {
        self.p.pull().map(|x| dbg!(x))
    }
}

impl<T: Debug> Pusher for Vec<T> {
    type Inner = Self;
    type PullItem = T;

    fn pull(&mut self) -> Option<Self::PullItem> {
        if self.is_empty() {
            return None;
        }
        Some(self.remove(0))
    }
}

impl<'a, T: Pusher> Pusher for &'a mut T {
    type Inner = T::Inner;
    type PullItem = T::PullItem;

    fn pull(&mut self) -> Option<Self::PullItem> {
        (*self).pull()
    }
}

#[cfg(test)]
mod tests {
    use crate::bi::Pusher;

    #[test]
    fn best() {
        //        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        //        let items2 = vec![];
        //        let items3 = vec![];
        //        let mut pusher = items2.filter(|x| x % 4 == 0).fork(items3).map(|x| x * 2);
        //        for item in items {
        //            pusher.push(item);
        //        }
        //        let (items2, items3) = pusher.into_inner().into_inner();
        //        assert_eq!(items2, vec![4, 8, 12, 16]);
        //        assert_eq!(items3, vec![2, 4, 6, 8, 10, 12, 14, 16, 18]);

        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut items2 = vec![];
        let mut items3: Vec<i32> = vec![];
        let mut items4 = vec![];
        #[rustfmt::skip]
        let a = items
            .fork(|x| {
                x
                    .filter(|x| x % 2 == 0)
                    .fork(|x| {
                        x.map(|x| x * 1000)
                            .filter(|x| x % 4000 == 0)
                            .write(&mut items4)
                    })
                    .map(|x| dbg!(x))
                    .write(&mut items3)
            })
            .filter(|x| x % 4 == 0)
            //            .map(|x| dbg!(x))
            .map(|x| x * 2)
            .finish(&mut items2);
        assert_eq!(items2, vec![8, 16]);
        assert_eq!(items3, vec![2, 4, 6, 8]);
        assert_eq!(items4, vec![4000, 8000]);
    }
}

pub enum WriterError {
    NotReady,
}

// use std::{fmt::Debug, marker::PhantomData};
//
//// struct Push<T>(PhantomData<T>);
////
//// impl<T> Pusher for Push<T> {
////    type Inner = Self;
////    type PushItem = T;
////
////    fn push(&mut self, item: Self::PushItem) {
////        ()
////    }
////
////    fn into_inner(self) -> Self::Inner {
////        self
////    }
//// }
// struct Filter<P, F> {
//    pusher: P,
//    f: F,
//}
// impl<P, F> Pusher for Filter<P, F>
//    where
//        P: Pusher,
//        F: FnMut(&P::PullItem) -> bool,
//{
//    type Inner = P;
//    type PullItem = P::PullItem;
//    type PushItem = P::PushItem;
//
//    fn pull(&mut self) -> Option<Self::PullItem> {
//        while let Some(item) = self.pusher.pull() {
//            if (self.f)(&item) {
//                return Some(item);
//            }
//        }
//        None
//    }
//
//    fn push(&mut self, item: Self::PushItem) {
//        self.pusher.push(item);
//    }
//
//    fn into_inner(self) -> Self::Inner {
//        self.pusher
//    }
//}
// struct Map<P, F, T> {
//    pusher: P,
//    f: F,
//    _t: PhantomData<T>,
//}
// impl<P, F, T> Pusher for Map<P, F, T>
//    where
//        P: Pusher,
//        F: FnMut(P::PullItem) -> T,
//        T: Debug,
//{
//    type Inner = P;
//    type PullItem = T;
//    type PushItem = P::PushItem;
//
//    fn pull(&mut self) -> Option<Self::PullItem> {
//        self.pusher.pull().map(|x| (self.f)(x))
//    }
//
//    fn push(&mut self, item: Self::PushItem) {
//        dbg!(&item);
//        self.pusher.push(item);
//    }
//
//    fn into_inner(self) -> Self::Inner {
//        self.pusher
//    }
//}
// struct Fork<A, B> {
//    pusher: A,
//    forked: B,
//}
// impl<A, B> Pusher for Fork<A, B>
//    where
//        A: Pusher,
//        A::PullItem: Clone,
//        B: Pusher<PushItem = A::PullItem>,
//        A::PushItem: Clone,
//        A::PushItem: Into<A::PullItem>,
//{
//    type Inner = (A::Inner, B::Inner);
//    type PullItem = A::PullItem;
//    type PushItem = A::PushItem;
//
//    fn pull(&mut self) -> Option<Self::PullItem> {
//        let item = self.pusher.pull()?;
//        self.forked.push(item.clone());
//        Some(item)
//    }
//
//    fn push(&mut self, item: Self::PushItem) {
//        self.pusher.push(item.clone());
//        self.forked.push(item.into());
//    }
//
//    fn into_inner(self) -> Self::Inner {
//        (self.pusher.into_inner(), self.forked.into_inner())
//    }
//}
// pub trait Finalize<T> {
//    type Output;
//    fn from(other: T) -> Self::Output;
//}
// trait Pusher: Sized {
//    type Inner;
//    type PushItem: Debug;
//    type PullItem: Debug;
//    fn pull(&mut self) -> Option<Self::PullItem>;
//    fn push(&mut self, item: Self::PushItem);
//    fn into_inner(self) -> Self::Inner;
//    fn filter<F: FnMut(&Self::PullItem) -> bool>(self, f: F) -> Filter<Self, F> {
//        Filter { pusher: self, f }
//    }
//    fn map<F: FnMut(Self::PullItem) -> T, T>(self, f: F) -> Map<Self, F, T> {
//        Map {
//            pusher: self,
//            f,
//            _t: Default::default(),
//        }
//    }
//    fn fork<P: Pusher<PushItem = Self::PushItem>>(self, f: P) -> Fork<Self, P> {
//        Fork {
//            pusher: self,
//            forked: f,
//        }
//    }
//    fn write<T: Extend<Self::PullItem>>(mut self, writer: &mut T) {
//        while let Some(item) = self.pull() {
//            writer.extend([item]);
//        }
//    }
//}
// pub struct DebugPusher<P> {
//    p: P,
//}
// impl<P> Pusher for DebugPusher<P>
//    where
//        P: Pusher + Finalize<Self>,
//        P::PullItem: Debug,
//{
//    type Inner = P;
//    type PullItem = P::PullItem;
//    type PushItem = P::PushItem;
//
//    fn push(&mut self, item: Self::PushItem) {
//        self.p.push(item);
//    }
//
//    fn into_inner(self) -> Self::Inner {
//        self.p
//    }
//
//    fn pull(&mut self) -> Option<Self::PullItem> {
//        self.p.pull().map(|x| dbg!(x))
//    }
//}
// impl<T: Debug> Pusher for Vec<T> {
//    type Inner = Self;
//    type PullItem = T;
//    type PushItem = T;
//
//    fn pull(&mut self) -> Option<Self::PushItem> {
//        if self.is_empty() {
//            return None;
//        }
//        Some(self.remove(0))
//    }
//
//    fn push(&mut self, item: Self::PushItem) {
//        Vec::<T>::push(self, item);
//    }
//
//    fn into_inner(self) -> Self::Inner {
//        self
//    }
//}
// impl<'a, T: Pusher> Pusher for &'a mut T {
//    type Inner = T::Inner;
//    type PullItem = T::PullItem;
//    type PushItem = T::PushItem;
//
//    fn pull(&mut self) -> Option<Self::PullItem> {
//        (*self).pull()
//    }
//
//    fn push(&mut self, item: Self::PushItem) {
//        (*self).push(item)
//    }
//
//    fn into_inner(self) -> Self::Inner {
//        unimplemented!()
//    }
//}
//
//#[cfg(test)]
// mod tests {
//    use crate::bi::Pusher;
//
//    #[test]
//    fn best() {
//        //        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
//        //        let items2 = vec![];
//        //        let items3 = vec![];
//        //        let mut pusher = items2.filter(|x| x % 4 == 0).fork(items3).map(|x| x * 2);
//        //        for item in items {
//        //            pusher.push(item);
//        //        }
//        //        let (items2, items3) = pusher.into_inner().into_inner();
//        //        assert_eq!(items2, vec![4, 8, 12, 16]);
//        //        assert_eq!(items3, vec![2, 4, 6, 8, 10, 12, 14, 16, 18]);
//
//        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
//        let mut items2 = vec![];
//        let mut items3 = vec![];
//        #[rustfmt::skip]
//            let mut pusher = items
//            .fork(
//                items3
//                    .map(|x| dbg!(x))
//                    .filter(|x| x % 2 == 0)
//            )
//            .filter(|x| x % 4 == 0)
//
//            //            .map(|x| dbg!(x))
//            .map(|x| x * 2)
//            .write(&mut items2);
//        assert_eq!(items2, vec![8, 16]);
//        //        assert_eq!(items3, vec![4, 8]);
//    }
//}
