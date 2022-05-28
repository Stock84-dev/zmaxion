mod ext;
mod macros;

#[doc(hidden)]
pub use macros::__type_name_of_helper__;

#[doc(hidden)]
pub mod prelude {
    pub use crate::{
        ext::{HasTypeId, TypeName},
        ok, ok_break, ok_loop, some, some_break, some_loop, type_name_of,
    };
}

pub trait ReturnValue {
    fn default() -> Self;
}

impl<T: Default, E> ReturnValue for Result<T, E> {
    fn default() -> Self {
        Ok(T::default())
    }
}

impl ReturnValue for () {
    fn default() -> Self {
        ()
    }
}
