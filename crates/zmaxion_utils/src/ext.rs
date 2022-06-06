use std::any::TypeId;

use crate::prelude::*;

pub trait TypeName {
    fn type_name() -> &'static str;
}

impl<T: ?Sized> TypeName for T {
    fn type_name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait HasTypeId {
    fn id() -> TypeId;
}

impl<T: ?Sized + 'static> HasTypeId for T {
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

pub trait OptionIntoResultExt {
    type Nullable;
    fn some(self) -> AnyResult<Self::Nullable>;
}

impl<T> OptionIntoResultExt for Option<T> {
    type Nullable = T;

    fn some(self) -> AnyResult<Self::Nullable> {
        self.ok_or(anyhow!("called `Option::unwrap()` on a `None` value"))
    }
}
