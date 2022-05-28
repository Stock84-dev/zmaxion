#[macro_export]
macro_rules! some {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return $crate::ReturnValue::default(),
        }
    };
}

#[macro_export]
macro_rules! some_loop {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! some_break {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => break,
        }
    };
}

#[macro_export]
macro_rules! ok {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => return $crate::ReturnValue::default(),
        }
    };
}

#[macro_export]
macro_rules! ok_loop {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => continue,
        }
    };
}

#[macro_export]
macro_rules! ok_break {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => break,
        }
    };
}

#[macro_export]
macro_rules! type_name_of {
    ($e:expr $(,)?) => {{
        let it = [];
        #[allow(unreachable_code)]
        {
            if false {
                loop {} // disables borrowck and dropck
                (&mut { it })[0] = &$e; // nudges type inference
            }
        }
        $crate::__type_name_of_helper__(it)
    }};
}
pub(in crate) use type_name_of;

#[doc(hidden)]
pub fn __type_name_of_helper__<T>(_: [&T; 0]) -> &'static str {
    ::core::any::type_name::<T>()
}
