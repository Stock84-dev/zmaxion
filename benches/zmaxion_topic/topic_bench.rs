use std::{
    any::TypeId,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use arc_swap::ArcSwap;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fixedbitset::FixedBitSet;
use futures::{future::BoxFuture, stream::FuturesUnordered, FutureExt, StreamExt};
// use zmaxion_core::prelude::*;
use parking_lot::{Mutex as PMutex, RwLock as PRwLock, *};
use rand::{prelude::SmallRng, RngCore, SeedableRng};
use spin::{Mutex as SpinMutex, RwLock as SpinRwLock};
use tokio::{
    runtime::Runtime,
    sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock},
    task::JoinHandle,
};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[rustfmt::skip]
 fn criterion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let power = 12;
    let other: Vec<_> = (0..(1 << 5)).into_iter().collect();
    let original: Vec<_> = (0..(1 << power)).into_iter().collect();
    let raw: Vec<_> = (0..(1 << power)).into_iter().collect();
//    let mut queue_in = Vec::with_capacity(raw.len());
    let queue_out: Vec<_> = (0..(1 << power)).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(|x| Arc::new(x)).collect();
    c.bench_function("raw", |b| b.iter(|| process_raw(black_box(&raw), black_box(&other))));
//    c.bench_function("queued", |b| b.iter(|| queued(black_box(&raw), black_box(&mut queue_in),
// black_box(&queue_out), black_box(&other))));    c.bench_function("ref", |b| b.iter(||
// process_ref(black_box(&vref), black_box(&other))));    c.bench_function("box", |b| b.iter(||
// process_box(black_box(&vbox), black_box(&other))));
    c.bench_function("arc", |b| b.iter(|| process_arc(black_box(&varc), black_box(&other))));

    let map = |x| Mutex::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_mutex", |b| b.iter(|| process_raw_mutex(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_mutex", |b| b.iter(||
// process_ref_mutex(black_box(&vref), black_box(&other))));    c.bench_function("box_mutex", |b|
// b.iter(|| process_box_mutex(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_mutex", |b| b.iter(|| process_arc_mutex(black_box(&varc),
 black_box(&other))));

    let map = |x| RwLock::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_rw", |b| b.iter(|| process_raw_rw(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_rw", |b| b.iter(||
// process_ref_rw(black_box(&vref), black_box(&other))));    c.bench_function("box_rw", |b|
// b.iter(|| process_box_rw(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_rw", |b| b.iter(|| process_arc_rw(black_box(&varc),
 black_box(&other))));

    let map = |x| SpinMutex::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_mutex_spin", |b| b.iter(|| process_spin_raw_mutex(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_mutex_spin", |b| b.iter(||
// process_spin_ref_mutex(black_box(&vref), black_box(&other))));    c.bench_function("
// box_mutex_spin", |b| b.iter(|| process_spin_box_mutex(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_mutex_spin", |b| b.iter(|| process_spin_arc_mutex(black_box(&varc),
 black_box(&other))));

    let map = |x| SpinRwLock::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_rw_spin", |b| b.iter(|| process_spin_raw_rw(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_rw_spin", |b| b.iter(||
// process_spin_ref_rw(black_box(&vref), black_box(&other))));    c.bench_function("box_rw_spin",
// |b| b.iter(|| process_spin_box_rw(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_rw_spin", |b| b.iter(|| process_spin_arc_rw(black_box(&varc),
 black_box(&other))));

    let map = |x| PMutex::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_mutex_p", |b| b.iter(|| process_raw_mutex_p(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_mutex_p", |b| b.iter(||
// process_ref_mutex_p(black_box(&vref), black_box(&other))));    c.bench_function("box_mutex_p",
// |b| b.iter(|| process_box_mutex_p(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_mutex_p", |b| b.iter(|| process_arc_mutex_p(black_box(&varc),
 black_box(&other))));

    let map = |x| PRwLock::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_rw_p", |b| b.iter(|| process_raw_rw_p(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_rw_p", |b| b.iter(||
// process_ref_rw_p(black_box(&vref), black_box(&other))));    c.bench_function("box_rw_p", |b|
// b.iter(|| process_box_rw_p(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_rw_p", |b| b.iter(|| process_arc_rw_p(black_box(&varc),
 black_box(&other))));

    let map = |x| AsyncMutex::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_rw_p", |b| b.iter(|| process_raw_rw_p(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_rw_p", |b| b.iter(||
// process_ref_rw_p(black_box(&vref), black_box(&other))));    c.bench_function("box_rw_p", |b|
// b.iter(|| process_box_rw_p(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_async_mutex", |b| b.iter(|| process_arc_async_mutex(black_box(&varc),
 black_box(&other), black_box(&rt))));

    let map = |x| AsyncRwLock::new(x);
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();

//    c.bench_function("raw_rw_p", |b| b.iter(|| process_raw_rw_p(black_box(&raw),
// black_box(&other))));    c.bench_function("ref_rw_p", |b| b.iter(||
// process_ref_rw_p(black_box(&vref), black_box(&other))));    c.bench_function("box_rw_p", |b|
// b.iter(|| process_box_rw_p(black_box(&vbox), black_box(&other))));
    c.bench_function("arc_async_rw_lock", |b| b.iter(||
 process_arc_async_rw_lock(black_box(&varc), black_box(&other), black_box(&rt))));

    let map = |x| Arc::new(Arc::new(x));
    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    c.bench_function("arc_cloned", |b| b.iter(|| process_arc_cloned(black_box(&raw),
// black_box(&other))));
    let swd: Vec<_> = original.iter().cloned().map(|x|
 Arc::new(SpinMutex::new(Arc::new(x)))).collect();    let swdrw: Vec<_> =
 original.iter().cloned().map(|x| Arc::new(SpinRwLock::new(Arc::new(x)))).collect();
    let swdarc: Vec<_> = original.iter().cloned().map(|x|
 Arc::new(ArcSwap::new(Arc::new(Arc::new(x))))).collect();    let mut atomic: Vec<_> =
 original.iter().cloned().map(|x| (Arc::new(AtomicU8::new(0)), x)).collect();

//    c.bench_function("swap_dyn", |b| b.iter(|| swap_dyn(black_box(&swd), black_box(&other))));
//    c.bench_function("swap_dyn_rw", |b| b.iter(|| swap_dyn_rw(black_box(&swdrw),
// black_box(&other))));    c.bench_function("swap_dyn_arc", |b| b.iter(||
// swap_dyn_arc(black_box(&swdarc), black_box(&other))));    c.bench_function("swap_atomic", |b|
// b.iter(|| swap_atomic(black_box(&mut atomic), black_box(&original), black_box(&other))));
}

fn process_arc_async_mutex(
    values: &[Arc<AsyncMutex<u64>>],
    other: &[u64],
    rt: &Runtime,
) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = rt.block_on(x.lock());
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}

fn process_arc_async_rw_lock(
    values: &[Arc<AsyncRwLock<u64>>],
    other: &[u64],
    rt: &Runtime,
) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = rt.block_on(x.read());
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn parallel_futures(c: &mut Criterion) {
    trait Provider {
        fn run(&self) -> BoxFuture<()> {
            async {}.boxed()
        }
        fn run_tokio(&self, rt: &Runtime) -> JoinHandle<()> {
            rt.spawn(async {})
        }
    }

    impl Provider for () {
    }

    let provider: Box<dyn Provider> = Box::new(());
    c.bench_function("tokio", |b| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        b.iter(|| tokio_fn(black_box(&rt), black_box(&provider)));
        fn tokio_fn(rt: &Runtime, futs: &Box<dyn Provider>) {
            let n = 100000;
            let futs: Vec<_> = (0..n).map(|x| futs.run_tokio(rt)).collect();
            for f in futs {
                rt.block_on(f);
            }
        }
    });
    c.bench_function("unordered", |b| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        b.iter(|| unordered(black_box(&rt), black_box(&provider)));
        fn unordered(rt: &Runtime, futs: &Box<dyn Provider>) {
            let n = 100000;
            let mut futs: FuturesUnordered<BoxFuture<()>> = (0..n).map(|x| futs.run()).collect();
            while let Some(fut) = rt.block_on(futs.next()) {}
        }
    });
}
fn atomics(c: &mut Criterion) {
    let n = 1000000;
    let mut num = vec![0usize; n];
    let mut atomic_num = (0..n).map(|x| AtomicUsize::new(0)).collect::<Vec<_>>();
    c.bench_function("atomic", |b| b.iter(|| atomic(black_box(&atomic_num))));
    c.bench_function("raw", |b| b.iter(|| raw(black_box(&mut num))));

    fn atomic(n: &[AtomicUsize]) {
        for n in n {
            n.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn raw(n: &mut [usize]) {
        for n in n {
            *n += 1;
        }
    }
}

// criterion_group!(benches, alloc_bench);

// criterion_group!(benches, access);
// criterion_group!(benches, parallel_futures);
// criterion_group!(benches, atomics);
criterion_group!(benches, criterion_benchmark);
// criterion_main!(alloc);
criterion_main!(benches);

fn queued(values: &[u64], qin: &mut Vec<u64>, qout: &[u64], other: &[u64]) -> Vec<u64> {
    values.iter().map(|x| *x).collect()
}
fn process_raw(values: &[u64], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| other.iter().map(|y| y * *x).sum())
        .collect()
}
fn process_ref(values: &[&u64], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| other.iter().map(|y| y * *x).sum())
        .collect()
}
fn process_box(values: &[Box<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| other.iter().map(|y| y * **x).sum())
        .collect()
}
fn process_arc(values: &[Arc<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| other.iter().map(|y| y * **x).sum())
        .collect()
}
fn process_raw_mutex(values: &[Mutex<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock().unwrap();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_ref_mutex(values: &[&Mutex<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock().unwrap();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_box_mutex(values: &[Box<Mutex<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock().unwrap();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_arc_mutex(values: &[Arc<Mutex<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock().unwrap();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_raw_rw(values: &[RwLock<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read().unwrap();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_ref_rw(values: &[&RwLock<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read().unwrap();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_box_rw(values: &[Box<RwLock<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read().unwrap();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_arc_rw(values: &[Arc<RwLock<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read().unwrap();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_spin_raw_mutex(values: &[SpinMutex<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_spin_ref_mutex(values: &[&SpinMutex<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_spin_box_mutex(values: &[Box<SpinMutex<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_spin_arc_mutex(values: &[Arc<SpinMutex<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_spin_raw_rw(values: &[SpinRwLock<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_spin_ref_rw(values: &[&SpinRwLock<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_spin_box_rw(values: &[Box<SpinRwLock<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_spin_arc_rw(values: &[Arc<SpinRwLock<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_raw_mutex_p(values: &[PMutex<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_ref_mutex_p(values: &[&PMutex<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_box_mutex_p(values: &[Box<PMutex<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_arc_mutex_p(values: &[Arc<PMutex<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = *x.lock();
            other.iter().map(|y| y * g).sum()
        })
        .collect()
}
fn process_raw_rw_p(values: &[PRwLock<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_ref_rw_p(values: &[&PRwLock<u64>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_box_rw_p(values: &[Box<PRwLock<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_arc_rw_p(values: &[Arc<PRwLock<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * *g).sum()
        })
        .collect()
}
fn process_arc_cloned(values: &[Arc<Arc<u64>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let x = Arc::clone(x.deref());
            other.iter().map(|y| y * *x).sum()
        })
        .collect()
}
fn swap_dyn(values: &[Arc<SpinMutex<Arc<u64>>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.lock();
            other.iter().map(|y| y * **g).sum()
        })
        .collect()
}
fn swap_dyn_rw(values: &[Arc<SpinRwLock<Arc<u64>>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.read();
            other.iter().map(|y| y * **g).sum()
        })
        .collect()
}
fn swap_dyn_arc(values: &[Arc<ArcSwap<Arc<u64>>>], other: &[u64]) -> Vec<u64> {
    values
        .iter()
        .map(|x| {
            let g = x.load();
            other.iter().map(|y| y * ***g).sum()
        })
        .collect()
}
fn swap_atomic(values: &mut [(Arc<AtomicU8>, u64)], updated: &[u64], other: &[u64]) -> Vec<u64> {
    let mut vec = Vec::with_capacity(values.len());
    for (i, x) in values.iter_mut().enumerate() {
        let val = x.0.load(Ordering::SeqCst);
        if val == 1 {
            return Vec::new();
        }
        if val == 0 {
            x.1 = updated[i];
        }
        vec.push(other.iter().map(|y| y * x.1).sum());
    }
    vec
}
// fn alloc_bench(c: &mut Criterion) {
//    fn alloc_vec(n: usize) -> Vec<usize> {
//        let mut v = Vec::new();
//        for i in 0..n {
//            v.push(i);
//        }
//        v
//    }
//    fn hash<'a, T: 'static>(map: &'a mut HashMap<TypeId, Vec<usize>>, n: usize) -> &'a Vec<usize>
// {        let id = std::any::TypeId::of::<T>();
//        let v = map.entry(id).or_insert_with(|| {
//            let mut v = Vec::new();
//            for i in 0..n {
//                v.push(i);
//            }
//            v
//        });
//        v.clear();
//        for i in 0..n {
//            v.push(i);
//        }
//        v
//    }
//    fn no_hash(v: &mut Vec<usize>, n: usize) {
//        v.clear();
//        for i in 0..n {
//            v.push(i);
//        }
//    }
//    let n = 1;
//    c.bench_function("alloc_vec", |b| {
//        b.iter(|| black_box(alloc_vec(black_box(n))))
//    });
//    c.bench_function("hashed", |b| {
//        let mut map = HashMap::<TypeId, Vec<usize>>::new();
//        b.iter(|| {
//            black_box(hash::<u8>(black_box(&mut map), black_box(n)));
//            //            println!("hi");
//        })
//    });
//
//    c.bench_function("no_hash", |b| {
//        let mut v = vec![0; n];
//        b.iter(|| {
//            black_box(no_hash(black_box(&mut v), black_box(n)));
//            //            println!("hi");
//        })
//    });
//
//    //    c.bench_function("pooled", |b| {
//    //        b.iter(|| {
//    //            let v = POOL.get(&std::any::TypeId::of::<usize>()).unwrap();
//    //            let values = v.lock();
//    //            no_hash(&mut *values, n);
//    //            black_box(values);
//    //        })
//    //    });
//}

fn access(c: &mut Criterion) {
    let mut rng = SmallRng::seed_from_u64(0);
    let max_components = 1 << 10;
    let mut current: Vec<u32> = (0..max_components)
        .map(|x| rng.next_u32() % 100 / 99)
        .collect();
    current[0] = 0;
    let current_bitset =
        FixedBitSet::with_capacity_and_blocks(current.len(), current.iter().cloned());

    let needed_scalar: Vec<u32> = (0..(32))
        .map(|x| x / 1024)
        //        .map(|x| rng.random::<u32>() % max_components)
        .collect();
    let needed: Vec<u32> = (0..max_components)
        .map(|x| rng.next_u32() % 100 / 90)
        .collect();
    let needed_bitset =
        FixedBitSet::with_capacity_and_blocks(current.len(), needed.iter().cloned());
    let needed_scalar: Vec<usize> = needed_scalar.iter().map(|x| *x as usize).collect();

    c.bench_function("scalar", |b| {
        b.iter(|| scalar(black_box(&current_bitset), black_box(&needed_scalar)))
    });
    c.bench_function("bitset", |b| {
        b.iter(|| bitset(black_box(&current_bitset), black_box(&needed_bitset)))
    });
    //    c.bench_function("bitwise", |b| {
    //        b.iter(|| bitwise(black_box(&current), black_box(&needed)))
    //    });
    c.bench_function("bitwise_if", |b| {
        b.iter(|| bitwise_if(black_box(&current), black_box(&needed)))
    });

    fn scalar(current: &FixedBitSet, needed: &Vec<usize>) -> bool {
        for needed in needed {
            if current[*needed] {
                return false;
            }
        }
        return true;
    }
    fn bitset(current: &FixedBitSet, needed: &FixedBitSet) -> bool {
        current.is_disjoint(&needed)
    }
    fn bitwise(current: &Vec<u32>, needed: &Vec<u32>) -> bool {
        let mut sum = 0;
        for i in 0..current.len() {
            sum |= current[i] & needed[i];
        }
        sum == 0
    }
    fn bitwise_if(current: &Vec<u32>, needed: &Vec<u32>) -> bool {
        let mut i = 0;
        while i < current.len() && (current[i] & needed[i]) == 0 {
            i += 1;
        }
        i == current.len()
    }
}

mod b {
    use std::time::Duration;

    use tokio;

    #[test]
    fn a() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.spawn(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("finished");
        });
        std::thread::sleep(Duration::from_secs(2));
        rt.block_on(async {});
    }
}
