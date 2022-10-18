use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use dashmap::DashMap;
use downcast_rs::{impl_downcast, Downcast, DowncastSync};
use ergnomics::prelude::*;
use lazy_static::lazy_static;

use crate::prelude::*;

pub struct PoolArcMut<T: ?Sized + PoolItem>(PoolArc<T>);
impl<T: PoolItem> PoolArcMut<T> {
    pub unsafe fn to_dyn_shared(&self) -> PoolArc<dyn PoolItem> {
        PoolArc(self.0 .0.clone())
    }
}

impl<T: ?Sized + PoolItem> PoolArcMut<T> {
    pub unsafe fn as_shared(&self) -> &PoolArc<T> {
        &self.0
    }

    pub unsafe fn to_shared(&self) -> PoolArc<T> {
        self.0.clone()
    }

    pub fn into_shared(self) -> PoolArc<T> {
        self.0.clone()
    }
}

impl<T: ?Sized + PoolItem> Deref for PoolArcMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0 .0.deref()
    }
}

impl<T: ?Sized + PoolItem> DerefMut for PoolArcMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 .0.deref() as *const _ as *mut Self::Target) }
    }
}

pub struct PoolArc<T: ?Sized + PoolItem>(Arc<T>);

impl<T: PoolItem> PoolArc<T> {
    pub fn downcast_ref<U: PoolItem>(&self) -> &U
    where
        T: Downcast,
    {
        self.0.as_any().downcast_ref::<U>().unwrap()
    }
}

impl<T: ?Sized + PoolItem> Clone for PoolArc<T> {
    fn clone(&self) -> Self {
        PoolArc(self.0.clone())
    }
}

impl<T: ?Sized + PoolItem> Deref for PoolArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized + PoolItem> Drop for PoolArc<T> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.0) == 1 {
            POOL.insert(self.0.clone())
        }
    }
}

pub trait ArcPoolItem: Clone {
    fn upcast(&self) -> PoolArc<dyn PoolItem>;
}

impl<T: PoolItem> ArcPoolItem for PoolArc<T> {
    fn upcast(&self) -> PoolArc<dyn PoolItem> {
        PoolArc(self.0.clone())
    }
}

pub trait PoolItem: DowncastSync + Send + Sync + 'static {
    fn reset(&mut self);
}

impl_downcast!(sync PoolItem);

lazy_static! {
    pub static ref POOL: Pool = Pool::new();
}

struct TypedPool<T> {
    constructor: Box<dyn FnMut() -> T + Send + Sync>,
    items: Vec<T>,
}

pub struct Pool {
    data: DashMap<TypeId, Box<dyn AnyPool>>,
}

impl Pool {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }

    pub fn register<T: PoolItem>(
        &self,
        mut constructor: impl FnMut() -> T + Send + Sync + 'static,
    ) {
        self.data.insert(
            TypeId::of::<T>(),
            Box::new(TypedPool {
                constructor: Box::new(move || Arc::new((constructor)())),
                items: Vec::<Arc<T>>::new(),
            }),
        );
    }

    pub fn get<T: ?Sized + PoolItem>(&self) -> PoolArcMut<T> {
        let mut pool = self.mut_pool::<T>();
        let typed_pool = pool.downcast_mut::<TypedPool<Arc<T>>>().unwrap();
        let item = match typed_pool.items.pop() {
            None => (typed_pool.constructor)(),
            Some(item) => item,
        };
        unsafe {
            (&mut *(item.deref() as *const _ as *mut T)).reset();
        }

        PoolArcMut(PoolArc(item))
    }

    pub fn clear(&self) {
        for mut pool in self.data.iter_mut() {
            pool.clear();
        }
    }

    pub fn capacity(&self) -> usize {
        let mut capacity = 0;
        for pool in self.data.iter() {
            capacity += pool.capacity();
        }
        capacity
    }

    pub fn capacity_bytes(&self) -> usize {
        let mut capacity_bytes = 0;
        for pool in self.data.iter() {
            capacity_bytes += pool.capacity_bytes();
        }
        capacity_bytes
    }

    fn mut_pool<T: ?Sized + PoolItem>(
        &self,
    ) -> dashmap::mapref::one::RefMut<TypeId, Box<dyn AnyPool>> {
        let id = T::id();
        self.data
            .get_mut(&id)
            .expect_with(|| format!("{} is not registered inside mempool", T::type_name()))
    }

    // Arc must have only one reference
    fn insert<T: ?Sized + PoolItem>(&self, value: Arc<T>) {
        let mut pool = self.mut_pool::<T>();
        let typed_pool = pool.downcast_mut::<TypedPool<Arc<T>>>().unwrap();
        typed_pool.items.push(value);
    }
}

trait AnyPool: Downcast + Send + Sync + 'static {
    fn clear(&mut self);
    fn capacity(&self) -> usize;
    fn capacity_bytes(&self) -> usize;
}
impl_downcast!(AnyPool);

impl<T: Send + Sync + 'static> AnyPool for TypedPool<T> {
    fn clear(&mut self) {
        self.items.clear();
    }

    fn capacity(&self) -> usize {
        self.items.capacity()
    }

    fn capacity_bytes(&self) -> usize {
        self.items.capacity() * T::size()
    }
}

impl<T: Send + Sync + 'static> PoolItem for Vec<T> {
    fn reset(&mut self) {
        self.clear();
    }
}
