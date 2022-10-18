use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
};
mod helpers;

pub trait ResultExt {
    fn ignore(self);
}

impl<T, E> ResultExt for Result<T, E> {
    fn ignore(self) {
    }
}

pub trait OptionExt {
    type Nullable;
    fn some_mut<'a>(&'a mut self, callback: impl FnMut(&'a mut Self::Nullable));
    fn some_ref<'a>(&'a self, callback: impl FnMut(&'a Self::Nullable));
    fn some(self) -> Result<Self::Nullable, NoneError>;
    fn expect_with<F: FnOnce() -> E, E: Display>(self, f: F) -> Self::Nullable;
    fn ignore(self);
}

impl<T> OptionExt for Option<T> {
    type Nullable = T;

    fn some_mut<'a>(&'a mut self, callback: impl FnMut(&'a mut Self::Nullable)) {
        self.iter_mut().for_each(callback);
    }

    fn some_ref<'a>(&'a self, callback: impl FnMut(&'a Self::Nullable)) {
        self.iter().for_each(callback);
    }

    fn some(self) -> Result<Self::Nullable, NoneError> {
        self.ok_or(NoneError)
    }

    fn expect_with<F: FnOnce() -> E, E: Display>(self, f: F) -> Self::Nullable {
        match self {
            None => {
                panic!("{}", f());
            }
            Some(v) => v,
        }
    }

    fn ignore(self) {
    }
}

pub struct NoneError;

impl Debug for NoneError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("called `Option::unwrap()` on a `None` value")
    }
}

impl Display for NoneError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("called `Option::unwrap()` on a `None` value")
    }
}

impl Error for NoneError {
}

pub mod prelude {
    pub use crate::{IteratorExt, OptionExt, ResultExt, StaticSize, Transmutations};
}

pub trait Trivial: Send + Sync + 'static {}

impl<T> Trivial for T where T: Send + Sync + 'static
{
}

pub trait Transmutations: Sized {
    fn as_one_slice<'a>(&'a self) -> &'a [Self];
    fn as_one_slice_mut<'a>(&'a mut self) -> &'a mut [Self];
    fn as_u8_slice<'a>(&'a self) -> &'a [u8];
    unsafe fn as_u8_slice_mut<'a>(&'a mut self) -> &'a mut [u8];
    unsafe fn as_static<'a>(&'a self) -> &'static Self;
    unsafe fn as_static_mut<'a>(&'a mut self) -> &'static mut Self;
    unsafe fn as_mut_cast<'a>(&'a self) -> &'a mut Self;
    unsafe fn from_u8_slice<'a>(slice: &'a [u8]) -> &'a Self;
    unsafe fn from_u8_slice_mut<'a>(slice: &'a mut [u8]) -> &'a mut Self;
}

impl<T: Sized> Transmutations for T {
    fn as_one_slice<'a>(&'a self) -> &'a [Self] {
        unsafe { helpers::ptr_as_slice(self as *const Self, 1) }
    }

    fn as_one_slice_mut<'a>(&'a mut self) -> &'a mut [Self] {
        unsafe { helpers::ptr_as_slice_mut(self as *mut Self, 1) }
    }

    fn as_u8_slice<'a>(&'a self) -> &'a [u8] {
        unsafe { helpers::ptr_as_slice(self as *const Self, T::size()) }
    }

    unsafe fn as_u8_slice_mut<'a>(&'a mut self) -> &'a mut [u8] {
        helpers::ptr_as_slice_mut(self as *mut Self, T::size())
    }

    unsafe fn as_static<'a>(&'a self) -> &'static Self {
        std::mem::transmute(self)
    }

    unsafe fn as_static_mut<'a>(&'a mut self) -> &'static mut Self {
        std::mem::transmute(self)
    }

    unsafe fn as_mut_cast<'a>(&'a self) -> &'a mut Self {
        &mut *(self as *const _ as *mut Self)
    }

    unsafe fn from_u8_slice<'a>(slice: &'a [u8]) -> &'a Self {
        &*(slice.as_ptr() as *const Self)
    }

    unsafe fn from_u8_slice_mut<'a>(slice: &'a mut [u8]) -> &'a mut Self {
        &mut *(slice.as_mut_ptr() as *mut Self)
    }
}

pub trait StaticSize {
    /// Returns the size of a type in bytes.
    fn size() -> usize;
}

impl<T: Sized> StaticSize for T {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

pub trait IteratorExt: Iterator {
    fn into_vec(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }
}

impl<I: Iterator> IteratorExt for I {
}
