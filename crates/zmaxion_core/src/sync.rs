pub use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub type PrioMutex<T> = priomutex::Mutex<T>;
pub type PrioMutexGuard<'a, T> = priomutex::MutexGuard<'a, T>;
pub type SpinMutex<T> = spin::Mutex<T>;
pub type SpinMutexGuard<'a, T> = spin::MutexGuard<'a, T>;
