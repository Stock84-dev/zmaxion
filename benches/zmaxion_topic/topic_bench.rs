use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex, RwLock,
    },
};

use arc_swap::ArcSwap;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zmaxion_utils::prelude::{Mutex as PMutex, RwLock as PRwLock, *};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[rustfmt::skip]
fn criterion_benchmark(c: &mut Criterion) {
    let power = 10;
    let other: Vec<_> = (0..(1 << 5)).into_iter().collect();
    let original: Vec<_> = (0..(1 << power)).into_iter().collect();
//    let raw: Vec<_> = (0..(1 << power)).into_iter().collect();
//    let mut queue_in = Vec::with_capacity(raw.len());
//    let queue_out: Vec<_> = (0..(1 << power)).into_iter().collect();
//    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
//    let vbox: Vec<_> = original.iter().cloned().map(|x| Box::new(x)).collect();
//    let varc: Vec<_> = original.iter().cloned().map(|x| Arc::new(x)).collect();
//    c.bench_function("raw", |b| b.iter(|| process_raw(black_box(&raw), black_box(&other))));
//    c.bench_function("queued", |b| b.iter(|| queued(black_box(&raw), black_box(&mut queue_in), black_box(&queue_out), black_box(&other))));
//    c.bench_function("ref", |b| b.iter(|| process_ref(black_box(&vref), black_box(&other))));
//    c.bench_function("box", |b| b.iter(|| process_box(black_box(&vbox), black_box(&other))));
//    c.bench_function("arc", |b| b.iter(|| process_arc(black_box(&varc), black_box(&other))));
//
//    let map = |x| Mutex::new(x);
//    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
//    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
//    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();
//
//    c.bench_function("raw_mutex", |b| b.iter(|| process_raw_mutex(black_box(&raw), black_box(&other))));
//    c.bench_function("ref_mutex", |b| b.iter(|| process_ref_mutex(black_box(&vref), black_box(&other))));
//    c.bench_function("box_mutex", |b| b.iter(|| process_box_mutex(black_box(&vbox), black_box(&other))));
//    c.bench_function("arc_mutex", |b| b.iter(|| process_arc_mutex(black_box(&varc), black_box(&other))));
//
//    let map = |x| RwLock::new(x);
//    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
//    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
//    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();
//
//    c.bench_function("raw_rw", |b| b.iter(|| process_raw_rw(black_box(&raw), black_box(&other))));
//    c.bench_function("ref_rw", |b| b.iter(|| process_ref_rw(black_box(&vref), black_box(&other))));
//    c.bench_function("box_rw", |b| b.iter(|| process_box_rw(black_box(&vbox), black_box(&other))));
//    c.bench_function("arc_rw", |b| b.iter(|| process_arc_rw(black_box(&varc), black_box(&other))));
//
//    let map = |x| SpinMutex::new(x);
//    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
//    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
//    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();
//
//    c.bench_function("raw_mutex_spin", |b| b.iter(|| process_spin_raw_mutex(black_box(&raw), black_box(&other))));
//    c.bench_function("ref_mutex_spin", |b| b.iter(|| process_spin_ref_mutex(black_box(&vref), black_box(&other))));
//    c.bench_function("box_mutex_spin", |b| b.iter(|| process_spin_box_mutex(black_box(&vbox), black_box(&other))));
//    c.bench_function("arc_mutex_spin", |b| b.iter(|| process_spin_arc_mutex(black_box(&varc), black_box(&other))));
//
//    let map = |x| SpinRwLock::new(x);
//    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
//    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
//    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();
//
//    c.bench_function("raw_rw_spin", |b| b.iter(|| process_spin_raw_rw(black_box(&raw), black_box(&other))));
//    c.bench_function("ref_rw_spin", |b| b.iter(|| process_spin_ref_rw(black_box(&vref), black_box(&other))));
//    c.bench_function("box_rw_spin", |b| b.iter(|| process_spin_box_rw(black_box(&vbox), black_box(&other))));
//    c.bench_function("arc_rw_spin", |b| b.iter(|| process_spin_arc_rw(black_box(&varc), black_box(&other))));
//
//    let map = |x| PMutex::new(x);
//    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
//    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
//    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();
//
//    c.bench_function("raw_mutex_p", |b| b.iter(|| process_raw_mutex_p(black_box(&raw), black_box(&other))));
//    c.bench_function("ref_mutex_p", |b| b.iter(|| process_ref_mutex_p(black_box(&vref), black_box(&other))));
//    c.bench_function("box_mutex_p", |b| b.iter(|| process_box_mutex_p(black_box(&vbox), black_box(&other))));
//    c.bench_function("arc_mutex_p", |b| b.iter(|| process_arc_mutex_p(black_box(&varc), black_box(&other))));
//
//    let map = |x| PRwLock::new(x);
//    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    let vref: Vec<_> = raw.iter().collect::<Vec<_>>();
//    let vbox: Vec<_> = original.iter().cloned().map(map).map(|x| Box::new(x)).collect();
//    let varc: Vec<_> = original.iter().cloned().map(map).map(|x| Arc::new(x)).collect();
//
//    c.bench_function("raw_rw_p", |b| b.iter(|| process_raw_rw_p(black_box(&raw), black_box(&other))));
//    c.bench_function("ref_rw_p", |b| b.iter(|| process_ref_rw_p(black_box(&vref), black_box(&other))));
//    c.bench_function("box_rw_p", |b| b.iter(|| process_box_rw_p(black_box(&vbox), black_box(&other))));
//    c.bench_function("arc_rw_p", |b| b.iter(|| process_arc_rw_p(black_box(&varc), black_box(&other))));
//
//    let map = |x| Arc::new(Arc::new(x));
//    let raw: Vec<_> = (0..(1 << power)).map(map).into_iter().collect();
//    c.bench_function("arc_cloned", |b| b.iter(|| process_arc_cloned(black_box(&raw), black_box(&other))));
//
//    let swd: Vec<_> = original.iter().cloned().map(|x| Arc::new(SpinMutex::new(Arc::new(x)))).collect();
//    let swdrw: Vec<_> = original.iter().cloned().map(|x| Arc::new(SpinRwLock::new(Arc::new(x)))).collect();
//    let swdarc: Vec<_> = original.iter().cloned().map(|x| Arc::new(ArcSwap::new(Arc::new(Arc::new(x))))).collect();
    let mut atomic: Vec<_> = original.iter().cloned().map(|x| (Arc::new(AtomicU8::new(0)), x)).collect();
//
//    c.bench_function("swap_dyn", |b| b.iter(|| swap_dyn(black_box(&swd), black_box(&other))));
//    c.bench_function("swap_dyn_rw", |b| b.iter(|| swap_dyn_rw(black_box(&swdrw), black_box(&other))));
//    c.bench_function("swap_dyn_arc", |b| b.iter(|| swap_dyn_arc(black_box(&swdarc), black_box(&other))));
    c.bench_function("swap_atomic", |b| b.iter(|| swap_atomic(black_box(&mut atomic), black_box(&original), black_box(&other))));
}

criterion_group!(benches, criterion_benchmark);
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
