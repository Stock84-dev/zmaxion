use std::{any::TypeId, fmt::Display};

use crate::prelude::*;

pub trait TypeName {
    fn type_name() -> &'static str;
}

impl<T: ?Sized> TypeName for T {
    fn type_name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait TypeIdExt {
    fn id() -> TypeId;
}

impl<T: ?Sized + 'static> TypeIdExt for T {
    fn id() -> TypeId {
        TypeId::of::<Self>()
    }
}

pub trait BoolExt {
    fn map_true<F: FnOnce() -> T, T>(self, mapper: F) -> Option<T>;
    fn map_false<F: FnOnce() -> T, T>(self, mapper: F) -> Option<T>;
}

impl BoolExt for bool {
    fn map_true<F: FnOnce() -> T, T>(self, mapper: F) -> Option<T> {
        if self {
            Some(mapper())
        } else {
            None
        }
    }

    fn map_false<F: FnOnce() -> T, T>(self, mapper: F) -> Option<T> {
        (!self).map_true(mapper)
    }
}
