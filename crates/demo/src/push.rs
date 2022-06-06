use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
};

use slab::Slab;
use zmaxion_utils::prelude::*;

pub struct DebugWriter;

pub trait Writer<T: RouterItem> {
    fn write(&mut self, item: T);
}

impl<T: RouterItem + Debug> Writer<T> for DebugWriter {
    fn write(&mut self, item: T) {
        dbg!(item);
    }
}

pub trait RouterItem: Sized {
    type Get;
    type Meta;
    fn into(self) -> Self::Get;
    fn as_ref(&self) -> &Self::Get;
    fn as_mut(&mut self) -> &mut Self::Get;
    fn into_parts(self) -> (Self::Get, Self::Meta);
}

pub trait RouterItemConstructor<T> {
    type Meta;
    type Output;
    fn from_parts(item: T, meta: Self::Meta) -> Self::Output;
}

impl<T, U> RouterItemConstructor<T> for U {
    type Meta = ();
    type Output = T;

    fn from_parts(item: T, _meta: Self::Meta) -> Self::Output {
        item
    }
}

pub struct Filter<A, F> {
    generator: A,
    predicate: F,
}

impl<T> Router for Vec<T> {
    type Item = T;

    fn run_once(&mut self, mut f: impl FnOnce(Self::Item) -> RouterFlow) -> RouterFlow {
        if self.is_empty() {
            return RouterFlow::Break;
        }
        let item = self.remove(0);
        f(item)
    }
}

impl<T> RouterItem for T {
    type Get = T;
    type Meta = ();

    fn into(self) -> Self::Get {
        self
    }

    fn as_ref(&self) -> &Self::Get {
        self
    }

    fn as_mut(&mut self) -> &mut Self::Get {
        self
    }

    fn into_parts(self) -> (Self::Get, Self::Meta) {
        (self, ())
    }
}

impl<A, F> Router for Filter<A, F>
where
    A: Router,
    A::Item: RouterItem,
    F: FnMut(&<A::Item as RouterItem>::Get) -> bool,
{
    type Item = A::Item;

    fn run_once(&mut self, mut f: impl FnOnce(Self::Item) -> RouterFlow) -> RouterFlow {
        self.generator.run_once(|item| {
            if (self.predicate)(item.as_ref()) {
                (f)(item)
            } else {
                RouterFlow::Continue
            }
        })
    }
}

pub struct Map<G, M, T> {
    generator: G,
    map: M,
    _t: PhantomData<T>,
}

impl<G, M, T> Router for Map<G, M, T>
where
    G: Router,
    G::Item: RouterItem + RouterItemConstructor<T, Meta = <G::Item as RouterItem>::Meta>,
    M: FnMut(<G::Item as RouterItem>::Get) -> T,
{
    type Item = <G::Item as RouterItemConstructor<T>>::Output;

    fn run_once(&mut self, mut f: impl FnOnce(Self::Item) -> RouterFlow) -> RouterFlow {
        self.generator.run_once(|item| {
            let (item, meta) = item.into_parts();
            let item = (self.map)(item);
            f(G::Item::from_parts(item, meta))
        })
    }
}

pub struct Zip<A: Router, B: Router> {
    a: A,
    b: B,
    last_left: Option<A::Item>,
}

impl<A, B> Router for Zip<A, B>
where
    A: Router,
    B: Router,
{
    type Item = (A::Item, B::Item);

    fn run_once(&mut self, mut f: impl FnOnce(Self::Item) -> RouterFlow) -> RouterFlow {
        let maybe_item = self.last_left.take();
        let mut last_left = &mut self.last_left;
        let item = match maybe_item {
            None => {
                self.a.run_once(|x| {
                    *last_left = Some(x);
                    RouterFlow::Continue
                });
                match last_left.take() {
                    None => {
                        return RouterFlow::Break;
                    }
                    Some(item) => item,
                }
            }
            Some(item) => item,
        };
        self.b.run_once(move |x| {
            match (f)((item, x)) {
                RouterFlow::Continue => {}
                RouterFlow::Break => return RouterFlow::Break,
            }
            RouterFlow::Continue
        })
    }
}

pub struct Write<G, W> {
    generator: G,
    writer: W,
}

impl<G: Router, W: Writer<G::Item>> Router for Write<G, W> {
    type Item = G::Item;

    fn run_once(&mut self, f: impl FnOnce(Self::Item) -> RouterFlow) -> RouterFlow {
        self.generator.run_once(|x| {
            self.writer.write(x);
            RouterFlow::Continue
        })
    }
}

#[derive(Eq, PartialEq)]
pub enum RouterFlow {
    Continue,
    Break,
}

pub trait Router: Sized {
    type Item: RouterItem;

    fn run_once(&mut self, f: impl FnOnce(Self::Item) -> RouterFlow) -> RouterFlow;
    fn run(&mut self, mut f: impl FnMut(Self::Item) -> RouterFlow) -> RouterFlow {
        while self.run_once(&mut f) == RouterFlow::Continue {}
        RouterFlow::Break
    }
    fn filter<P>(self, f: P) -> Filter<Self, P>
    where
        P: FnMut(&<Self::Item as RouterItem>::Get) -> bool,
    {
        Filter {
            generator: self,
            predicate: f,
        }
    }

    fn map<P, T>(self, f: P) -> Map<Self, P, T>
    where
        P: FnMut(<Self::Item as RouterItem>::Get) -> T,
    {
        Map {
            generator: self,
            map: f,
            _t: Default::default(),
        }
    }

    fn zip<P>(self, generator: P) -> Zip<Self, P>
    where
        P: Router,
    {
        Zip {
            a: self,
            b: generator,
            last_left: None,
        }
    }

    fn write<W>(mut self, writer: &mut W)
    where
        W: Writer<Self::Item>,
    {
        self.run(|x| {
            writer.write(x);
            RouterFlow::Continue
        });
    }
}


#[cfg(test)]
mod tests {
    use crate::push::{DebugWriter, Router, RouterFlow};

    #[test]
    fn push() {
        let mut items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut items2 = vec![3, 3, 4, 4, 3, 3, 3, 3, 3];
        let mut writer = DebugWriter;
        let mut router = items
            .map(|x| x * 2)
            .zip(items2)
            //            .filter(|x| x.0 % 4 == 0)
            .map(|x| x.1 as f32 * 10.)
            .write(&mut writer);
        //        router.run(|x| {
        //            dbg!(x);
        //            RouterFlow::Continue
        //        });
    }
}
