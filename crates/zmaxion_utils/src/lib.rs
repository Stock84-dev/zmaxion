pub mod convert;
mod ext;
mod macros;
pub mod pool;
mod sync;

#[doc(hidden)]
pub use macros::__type_name_of_helper__;

pub mod prelude {
    pub use std::sync::Arc;

    pub use anyhow::{
        anyhow, bail, ensure, format_err, Chain, Context as AnyContext, Error as AnyError,
        Result as AnyResult,
    };
    pub use tracing::{
        debug, debug_span, enabled, error, error_span, event, event_enabled, info, info_span, span,
        span_enabled, trace, trace_span, warn, warn_span,
    };
    pub use tracing_futures::Instrument;

    pub use crate::{
        ext::{BoolExt, TypeIdExt, TypeName},
        ok, ok_break, ok_loop, some, some_break, some_loop,
        sync::*,
        type_name_of,
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
