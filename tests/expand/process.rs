use std::marker::PhantomData;

pub use zmaxion::core::markers::TopicReader;
use zmaxion::{core::markers::*, prelude::*};
use zmaxion_internal::core::models::PipeDeclaration;

struct Struct<'w, 's>(PhantomData<&'w &'s ()>);
trait Serialize {}
struct BevyReader<T>(T);
struct AckEvent<T>(T);
struct TraceEvent<T>(T);

//
//#[process]
// fn simple() {
//}
//
//#[process]
// fn arg(a: Struct) {
//}
//
//#[process]
// fn arg_ret(a: Struct) -> Result<i32, String> {
//    Ok(0)
//}
//
//#[process]
// fn arg_ret_impl(a: impl TopicReader<'s, i32>) -> Result<i32, String> {
//    Ok(0)
//}
//
//#[process]
// fn arg_ret_impl_async(a: impl TopicReader<'s, i32> + Async) -> Result<i32, String> {
//    Ok(0)
//}
//
#[process]
fn gen_arg_ret_impl_async<E: Serialize>(a: impl TopicReader<'s, E> + Async) -> Result<i32, String> {
    const FEATURES: PipeFeatures<BevyRuntimeMarker> = PipeFeatures {
        runtime: BevyRuntimeMarker,
    };
    Ok(0)
}

//#[process]
// fn gen_arg_ret_dyn_async<E: Serialize>(
//    a: &mut (dyn TopicReader<'s, E> + Async),
//) -> Result<i32, String> {
//    Ok(0)
//}

// pub fn register_arg_ret_impl_asynca<'a, 'b, 'w, 's, __P0: TopicReader<'s, i32> + Async>(
//    app: &'b mut zmaxion_internal::app::prelude::AppBuilder<'a>,
//) -> &'b mut zmaxion_internal::app::prelude::AppBuilder<'a> {
//    app.register_pipe(arg_ret_impl_async::<AsyncReader<AckEvent<TraceEvent<'s, i32>>>>);
//    app.register_pipe(arg_ret_impl_async::<AsyncReader<AckEvent<'s, i32>>>);
//    app.register_pipe(arg_ret_impl_async::<AsyncReader<'s, i32>>);
//    app.register_pipe(arg_ret_impl_async::<AsyncGroupReader<AckEvent<TraceEvent<'s, i32>>>>);
//    app.register_pipe(arg_ret_impl_async::<AsyncGroupReader<AckEvent<'s, i32>>>);
//    app.register_pipe(arg_ret_impl_async::<AsyncGroupReader<'s, i32>>);
//    app
//}
// fn arg_ret_impl_asynca<'w, 's, __P0: TopicReader<'s, i32> + Async>(a: __P0) -> Result<i32,
// String> {    Ok(0)
//}

// pub fn register_gen_arg_ret_impl_async<'a, 'b, 'w, 's, E: Serialize>(
//    app: &'b mut zmaxion_internal::app::prelude::AppBuilder<'a>,
//) -> &'b mut zmaxion_internal::app::prelude::AppBuilder<'a> {
//    let declaration = PipeDeclaration {
//        param_names: vec!["a".to_string()],
//        features: PipeFeatures {
//            runtime: BevyRuntimeMarker,
//        },
//    };
//    app.register_pipe(
//        gen_arg_ret_impl_async::<E, AsyncReader<'w, 's, AckEvent<TraceEvent<E>>>>,
//        declaration.clone(),
//    );
//    app.register_pipe(
//        gen_arg_ret_impl_async::<E, AsyncReader<'w, 's, AckEvent<E>>>,
//        declaration.clone(),
//    );
//    app.register_pipe(
//        gen_arg_ret_impl_async::<E, AsyncReader<'w, 's, E>>,
//        declaration.clone(),
//    );
//    app.register_pipe(
//        gen_arg_ret_impl_async::<E, AsyncGroupReader<'w, 's, AckEvent<TraceEvent<E>>>>,
//        declaration.clone(),
//    );
//    app.register_pipe(
//        gen_arg_ret_impl_async::<E, AsyncGroupReader<'w, 's, AckEvent<E>>>,
//        declaration.clone(),
//    );
//    app.register_pipe(
//        gen_arg_ret_impl_async::<E, AsyncGroupReader<'w, 's, E>>,
//        declaration.clone(),
//    );
//    app
//}
// fn gen_arg_ret_impl_async<'w, 's, E: Serialize, __P0: TopicReader<'s, E> + Async>(
//    a: __P0,
//) -> Result<i32, String> {
//    #[allow(unused)]
//    const FEATURES: PipeFeatures<BevyRuntimeMarker> = PipeFeatures {
//        runtime: BevyRuntimeMarker,
//    };
//    Ok(0)
//}
