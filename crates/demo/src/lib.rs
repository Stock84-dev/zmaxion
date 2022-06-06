mod bi;
pub mod push;
// use std::{
//    any::{Any, TypeId},
//    collections::HashMap,
//    fmt::Debug,
//    marker::PhantomData,
//};
// use slab::Slab;
// use zmaxion_utils::prelude::*;
//
//#[derive(Clone)]
// pub struct JoinedItem<A, B> {
//    a: A,
//    b: B,
//}
// pub struct JoinedMetaItem<A, B> {
//    a: Meta<A>,
//    b: Meta<B>,
//}
// pub trait Joiner<U> {
//    type Output: RouterItem;
//    fn join(self, u: U) -> Self::Output;
//}
// pub trait Wrapper<T> {
//    type Wrapped: RouterItem;
//    fn wrap(self, t: T) -> Self::Wrapped;
//}
// pub trait RouterItem: Sized {
//    type Get;
//    fn get(&self) -> Self::Get;
//}
// struct Dependency {
//    reader_id: usize,
//    generation: u64,
//}
// struct Meta<T> {
//    data: usize,
//    generation: u64,
//    deps: Vec<Dependency>,
//    _t: PhantomData<T>,
//}
// struct Arena<T> {
//    data: Slab<T>,
//}
// struct Messages {
//    arena: HashMap<TypeId, Box<dyn Any>>,
//}
// pub struct Context {
//    n_readers: usize,
//}
// pub struct Join<A, B> {
//    a: A,
//    b: B,
//}
// impl<A, B> Router for Join<A, B>
// where
//    A: Router,
//    B: Router,
//    A::Item: RouterItem + Joiner<B::Item>,
//    B::Item: RouterItem,
//{
//    type Item = <A::Item as Joiner<B::Item>>::Output;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        let a = self.a.next()?;
//        let b = self.b.next()?;
//        Some(a.join(b))
//    }
//}
// pub struct Filter<A, F> {
//    a: A,
//    f: F,
//}
// impl<A, F> Router for Filter<A, F>
// where
//    A: Router,
//    A::Item: RouterItem,
//    F: FnMut(<A::Item as RouterItem>::Get) -> bool,
//{
//    type Item = A::Item;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        loop {
//            let value = self.a.next()?;
//            if (self.f)(value.get()) {
//                return Some(value);
//            }
//        }
//    }
//}
// pub struct Map<A, F, T> {
//    a: A,
//    f: F,
//    _t: PhantomData<T>,
//}
// impl<A, F, T> Router for Map<A, F, T>
// where
//    A: Router,
//    A::Item: Wrapper<T>,
//    F: FnMut(A::Item) -> T,
//{
//    type Item = <A::Item as Wrapper<T>>::Wrapped;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        let value = self.a.next()?;
//        let mapped = (self.f)(value);
//        Some(value.wrap(mapped))
//    }
//}
// pub struct Write<A, B> {
//    a: A,
//    writer: B,
//}
// impl<A, B> Router for Write<A, B>
// where
//    A: Router,
//    B: Writer<A::Item>,
//    A::Item: Clone,
//{
//    type Item = A::Item;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        let item = self.a.next()?;
//        self.writer.write(item.clone());
//        Some(item)
//    }
//}
// pub trait Router: Sized {
//    type Item: RouterItem;
//    fn next(&mut self) -> Option<Self::Item>;
//    //    fn join<R>(self, other: R) -> Zip<Self, R>;
//    //    fn fork<R>(self) -> (Self, Self)
//    fn filter<F: FnMut(&Self::Item) -> bool>(self, filter: F) -> Filter<Self, F> {
//        Filter { a: self, f: filter }
//    }
//    fn map<F: FnMut(Self::Item) -> T, T>(self, map: F) -> Map<Self, F, T> {
//        Map {
//            a: self,
//            f: map,
//            _t: Default::default(),
//        }
//    }
//    fn write<W: Writer<Self::Item>>(self, writer: W) -> Write<Self, W> {
//        Write { a: self, writer }
//    }
//}
// pub trait Writer<T> {
//    fn write(&mut self, value: T);
//}
// struct FooReader;
//
// impl Router for FooReader {
//    type Item = ();
//
//    fn next(&mut self) -> Option<Self::Item> {
//        Some(())
//    }
//}
// struct FooWriter;
//
// impl<T: Debug> Writer<T> for FooWriter {
//    fn write(&mut self, value: T) {
//        dbg!(value);
//    }
//}
//
//#[cfg(test)]
// mod tests {
//    use crate::{FooReader, FooWriter, Router};
//
//    #[test]
//    fn test() {
//        let mut reader = FooReader;
//        let mut writer = FooWriter;
//        let mut router = reader.map(|_| 42).write(writer);
//        while let Some(r) = router.next() {
//            dbg!(r);
//        }
//    }
//}

// use std::{
//    future::Future,
//    marker::PhantomData,
//    pin::Pin,
//    sync::Arc,
//    task::{Context, Poll},
//};
// use bevy_ecs::prelude::*;
// use futures_lite::future;
//
// struct AsyncRes<T: 'static>(&'static &'static T);
//
////#[async_system]
//// async fn async_system(name: AsyncRes<String>) {
////    println!("before");
////    {
////        let guard = name.get();
////        dbg!(guard.deref());
////    }
////    MyFuture(false).await;
////    println!("after");
////    let guard = name.get();
////    dbg!(guard.deref());
//// }
// fn wrapper_system<'w>(
//    mut query: Query<(Entity, Option<&mut BevyFuture<()>>)>,
//    mut res: ResMut<'w, Vec<String>>,
//    mut commands: Commands,
//    mut polled: Local<u8>,
//) {
//    match query.get_single_mut() {
//        Ok((id, Some(mut fut))) => {
//            let captured_reference = &*fut.captured.0;
//            if *polled == 1 {
//                println!("changing");
//                res.push(String::from("This string has been changed and moved"));
//                unsafe {
//                    *(captured_reference as *const _ as *mut &String) = &res[1];
//                }
//            }
//            match future::block_on(future::poll_once(&mut fut.future)) {
//                None => {}
//                Some(_) => {
//                    commands.entity(id).despawn();
//                }
//            }
//            *polled += 1;
//        }
//        _ => {
//            println!("spawning another future");
//            let reference = &res[0];
//            commands.spawn().insert(desugared_async_system(reference));
//        }
//    }
//}
// fn desugared_async_system<'a>(item: &'a String) -> BevyFuture<()> {
//    let mut item = AsyncRef::new(item);
//    BevyFuture {
//        captured: item.clone(),
//        future: Box::pin(async move {
//            let must_be_cloned;
//            {
//                let guard = item.get();
//                must_be_cloned = guard.deref().clone();
//            }
//            MyFuture(&must_be_cloned, false).await;
//        }),
//    }
//}
//
//#[derive(Component)]
// struct BevyFuture<F> {
//    captured: AsyncRef<String>,
//    future: Pin<Box<dyn Future<Output = F> + Send + Sync>>,
//}
// struct AsyncRef<T: 'static>(Arc<&'static T>);
//
// impl<T> Clone for AsyncRef<T> {
//    fn clone(&self) -> Self {
//        Self(self.0.clone())
//    }
//}
// struct Guard<'a, T> {
//    reference: &'a T,
//    _t: PhantomData<*const ()>,
//}
// impl<'a, T> Guard<'a, T> {
//    pub fn deref(&'a self) -> &'a T {
//        self.reference
//    }
//}
// impl<T: 'static> AsyncRef<T> {
//    pub fn new(value: &T) -> Self {
//        // change to 'static lifetime
//        Self(Arc::new(unsafe { std::mem::transmute(value) }))
//    }
//
//    pub fn get<'a>(&'a mut self) -> Guard<'a, T> {
//        let a = *self.0;
//        Guard {
//            reference: *self.0,
//            _t: Default::default(),
//        }
//    }
//}
// struct MyFuture<'a, T>(&'a T, bool);
// impl<'a, T> Future for MyFuture<'a, T> {
//    type Output = ();
//
//    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//        if self.1 {
//            return Poll::Ready(());
//        }
//        self.1 = true;
//        Poll::Pending
//    }
//}
//
//#[cfg(test)]
// mod tests {
//    use std::{
//        future::Future,
//        pin::Pin,
//        task::{Context, Poll},
//        time::Instant,
//    };
//
//    use bevy::prelude::App;
//    use futures_lite::FutureExt;
//    use tokio::runtime::Runtime;
//
//    use crate::wrapper_system;
//
//    #[test]
//    fn test() {
//        let mut app = App::empty();
//        app.add_default_stages()
//            .insert_resource(vec!["Hello world".to_string()]);
//        app.add_system(wrapper_system);
//        app.update();
//        app.update();
//        app.update();
//        app.update();
//        app.update();
//        app.update();
//    }
//
//    pin_project_lite::pin_project! {
//        struct MyFuture<F> {
//            #[pin]
//            fut: F,
//            now: Instant,
//        }
//    }
//
//    impl<F: Future> Future for MyFuture<F> {
//        type Output = F::Output;
//
//        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//            let this = self.project();
//            std::thread::sleep(std::time::Duration::from_millis(7500));
//
//            let a = this.fut.poll(cx);
//            println!("{}", this.now.elapsed().as_millis());
//            *this.now = Instant::now();
//            a
//        }
//    }
//
//    #[test]
//    fn polling() {
//        let mut runtime = Runtime::new().unwrap();
//        runtime.block_on(async {
//            let now = Instant::now();
//            let fut = MyFuture {
//                fut: reqwest::get("https://www.rust-lang.org"),
//                now: Instant::now(),
//            }
//            .await
//            .unwrap();
//            // 14 polls across 700 ms
//            println!("text");
//            let fut = MyFuture {
//                now: Instant::now(),
//                fut: fut.text(),
//            }
//            .await
//            .unwrap();
//        });
//    }
//}
