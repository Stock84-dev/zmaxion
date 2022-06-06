#![allow(unused_parens)]

use std::{
    future::Future,
    io::Read,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bevy::ecs::{
    all_tuples,
    schedule::{IntoSystemDescriptor, SystemDescriptor},
    system::{
        ConfigurableSystem, SystemMeta, SystemParam, SystemParamFetch, SystemParamFunction,
        SystemParamState,
    },
};
use factory::*;
use param::*;
use paste::paste;
use smallvec::SmallVec;

use crate::{
    error::{ErrorEvent, Errors, ErrorsState, PipeParamError},
    models::PipeKind,
    pipeline::SpawnPipelineInner,
    prelude::*,
    topic::TopicId,
};

pub mod factory;
pub mod param;

mod resources {
    use crate::{pipe::factory::PipeFactoryContainer, prelude::*};
    pub struct PipeDefs(pub HashMap<&'static str, PipeFactoryContainer>);
}

#[async_trait]
pub trait Pipe: Send + Sync {
    fn system(&self) -> Option<SystemDescriptor>;
}

pub struct BevyPipe<Params, Fetch, F, Out, Marker> {
    container: SystemPipe<F, Params, Out>,
    fetch: Fetch,
    errors_state: Option<ErrorsState>,
    _params: PhantomData<Params>,
    _out: PhantomData<Out>,
    _marker: PhantomData<Marker>,
}

impl<Fetch, Params, F, Out, Marker> BevyPipe<Params, Fetch, F, Out, Marker> {
    pub fn new(f: F, fetch: Fetch, errors: Option<ErrorsState>) -> Self {
        Self {
            container: SystemPipe {
                f,
                _t: Default::default(),
            },
            fetch,
            errors_state: errors,
            _params: Default::default(),
            _out: Default::default(),
            _marker: Default::default(),
        }
    }
}

pub struct AsyncPipe<Params, Fetch, F, Fut, Out> {
    despawn: Arc<AtomicBool>,
    _f: PhantomData<fn() -> (F, Fetch, Fut, Out, Params)>,
}

impl<Fetch, Params, F, Fut, Out> Drop for AsyncPipe<Params, Fetch, F, Fut, Out> {
    fn drop(&mut self) {
        self.despawn.store(true, Ordering::SeqCst);
    }
}

impl<Params, Fetch, F, Fut, Out> Pipe for AsyncPipe<Params, Fetch, F, Fut, Out> {
    fn system(&self) -> Option<SystemDescriptor> {
        None
    }
}

macro_rules! impl_pipe_observer (
    ($($param: ident),*) => {
        impl<$($param: PipeObserver),*> PipeObserver for ($($param,)*) {
            fn pipe_executing(&mut self) {
                let ($($param,)*) = self;
                $(
                    $param.pipe_executing();
                )*
            }

            fn pipe_executed(&mut self) {
                let ($($param,)*) = self;
                $(
                    $param.pipe_executed();
                )*
            }
        }
    };
);

pub struct SystemPipe<F, Params, Out> {
    f: F,
    _t: PhantomData<fn() -> (Params, Out)>,
}

impl<F: Clone, Params, Out> Clone for SystemPipe<F, Params, Out> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _t: Default::default(),
        }
    }
}

macro_rules! impl_system_pipe (
    ($($param: ident),*) => {
        impl<F, Out, $($param,)* Marker> SystemParamFunction<(), Out, ($($param,)*), Marker>
            for SystemPipe<F, ($($param,)*), Out>
        where
            for<'a> &'a mut F:
                FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Out
                + FnMut($($param),*) -> Out,
            F: Clone
                + Send
                + Sync
                + 'static,
            ($($param,)*): SystemParam<Fetch = ($(<$param as PipeParam>::Fetch,)*)> + PipeParam,
            Out: Send + Sync + 'static,
            $(
                <$param as SystemParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + SystemParamState<Config = Option<<$param as PipeParam>::Fetch>>
                    + PipeObserver,
                $param: SystemParam<Fetch = <$param as PipeParam>::Fetch>
                    + PipeParam
                    + Send
                    + Sync
                    + 'static,
                <$param as PipeParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + PipeParamState
                    + SystemParamFetch<'static, 'static>
                    + Clone,
            )*
        {
            unsafe fn run(
                &mut self,
                input: (),
                state: &mut <($($param,)*) as SystemParam>::Fetch,
                system_meta: &SystemMeta,
                world: &World,
                change_tick: u32,
            ) -> Out {
                #![allow(non_snake_case)]
                let ($($param,)*) = &mut *state;
                $($param.pipe_executing();)*
                let out;
                {
                    $(
                        // SAFETY: we need 'static lifetime because PipeParamFetch is implemented for
                        // specific lifetime, thus for<'a> T: PipeParamFetch<'a> doesn't
                        // work The only way to ensure that PipeParamFetch::Item is F0 is
                        // through specific lifetime State valid untill this function ends
                        // and we are using it only inside of it
                        let $param: &'static mut <$param as PipeParam>::Fetch =
                            std::mem::transmute($param);
                    )*
                    // Yes, this is strange, but rustc fails to compile this impl
                    // without using this function.
                    #[allow(clippy::too_many_arguments)]
                    fn call_inner<Out, $($param,)*>(
                        mut f: impl FnMut($($param,)*)->Out,
                        $($param: $param,)*
                    )->Out{
                        f($($param,)*)
                    }
                    out = call_inner(&mut self.f,
                        $(<<$param as PipeParam>::Fetch as PipeParamFetch>::get_param(
                            $param
                        )),*
                    );
                }
                let ($($param,)*) = &mut *state;
                $($param.pipe_executed();)*
                out
            }
        }

    };
);

macro_rules! impl_pipe (
    ($($out: ident)?; $($param: ident),*) => {
        impl<F $(,$out)? $(,$param)*> Pipe for BevyPipe<($($param,)*), ($(<$param as PipeParam>::Fetch,)*), F, ($(AnyResult<$out>)?), ()>
        where
            for<'a> &'a mut F:
                FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> ($(AnyResult<$out>)?)
                + FnMut($($param),*) -> ($(AnyResult<$out>)?),
            F: Clone
                + Send
                + Sync
                + 'static,
            ($($param,)*): SystemParam<Fetch = ($(<$param as PipeParam>::Fetch,)*)> + PipeParam,
            $($out: Send + Sync + 'static,)?
            $(
                <$param as SystemParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + SystemParamState<Config = Option<<$param as PipeParam>::Fetch>>
                    + PipeObserver,
                $param: SystemParam<Fetch = <$param as PipeParam>::Fetch>
                    + PipeParam
                    + Send
                    + Sync
                    + 'static,
                <$param as PipeParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + PipeParamState
                    + SystemParamFetch<'static, 'static>
                    + Clone,
            )*
        {
            fn system(&self) -> Option<SystemDescriptor> {
                #![allow(non_snake_case)]
                let system = ConfigurableSystem::<_, _, _, ()>::config(self.container.clone(), |x| {
                        let ($(paste! {[<self_ $param>]} ,)*) = self.fetch.clone();
                        let ($(paste! {[<x_ $param>]} ,)*) = x;

                        $(*paste! {[<x_ $param>]} = paste! {[<self_ $param>]}.configure();)*
                });
                $(
                    let errors_state = self.errors_state.clone();
                    let system = system.chain(move |result: In<AnyResult<Out>>| {
                        let errors = errors_state.get_param();
                        crate::error::handle_errors::<$out>(result, errors_state)
                    });
                )?
                Some(system.into_descriptor())
            }
        }
    };
);

pub(crate) trait AsyncPipeTrait<Coroutine, State, Params, Fut> {
    fn new(f: Coroutine, state: State, errors_state: Option<ErrorsState>) -> Self;
}

macro_rules! impl_async_pipe (
    ($($out: ident)?; $($param: ident),*) => {
        #[async_trait]
        impl<F, $($param,)* Fut, $($out,)?>
            AsyncPipeTrait<F, ($(<$param as PipeParam>::Fetch,)*), ($($param,)*), Fut> for
            AsyncPipe<($($param,)*), ($(<$param as PipeParam>::Fetch,)*), F, Fut, ($(AnyResult<$out>,)?)>
        where
            F: FnMut($($param),*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = ($(AnyResult<$out>)?)> + Send + Sync + 'static,
            $($out: Send + Sync + 'static,)?
            $(
                $param: PipeParam + Send + Sync + 'static,
                <$param as PipeParam>::Fetch: PipeParamFetch<'static, Item = $param>
                    + PipeParamState
                    + PipeObserver
                    + Send
                    + Sync,
            )*
        {
            fn new(mut f: F, fetch: ($(<$param as PipeParam>::Fetch,)*), errors_state: Option<ErrorsState>) -> Self {
                #![allow(non_snake_case)]
                let despawn = Arc::new(AtomicBool::new(false));
                let despawn2 = despawn.clone();
                zmaxion_rt::spawn(async move {
                    $(
                        let _out: $out;
                        let errors = Errors::from(errors_state.as_ref().unwrap());
                    )?
                    let ($(mut $param,)*) = fetch;
                    loop {
                        {
                            // SAFETY: we need 'static lifetime because PipeParamFetch is implemented for
                            // specific lifetime, thus for<'a> T: PipeParamFetch<'a> doesn't
                            // work The only way to ensure that PipeParamFetch::Item is F0 is
                            // through specific lifetime State valid untill this function ends
                            // and we are using it only inside of it
                            $(
                                let $param = &mut $param;
                                $param.pipe_executing();
                                let $param: &'static mut $param::Fetch = unsafe {
                                    std::mem::transmute($param)
                                };
                            )*
                            let result = f($(<<$param as PipeParam>::Fetch>::get_param($param)),*).await;
                            $(
                                let _out: $out;
                                errors.handle(result);
                            )?
                        }
                        $(
                            $param.pipe_executed();
                        )*
                        if (*despawn).load(Ordering::SeqCst) {
                            break;
                        }
                    }
                });
                Self {
                    despawn: despawn2,
                    _f: Default::default(),
                }
            }
        }
    };
);

macro_rules! impl_all (
    ($($param: ident),*) => {
        impl_pipe_param!($($param),*);
        impl_pipe_param_fetch!($($param),*);
        impl_build_params!($($param),*);

        impl_async_pipe!(;$($param),*);
        impl_async_pipe!(Out;$($param),*);
        factory::impl_async_pipe_factory!(;$($param),*);
        factory::impl_async_pipe_factory!(Out;$($param),*);
        factory::impl_into_async_pipe_factory!(;$($param),*);
        factory::impl_into_async_pipe_factory!(Out;$($param),*);

        impl_pipe!(;$($param),*);
        impl_pipe!(Out;$($param),*);
        factory::impl_pipe_factory!(;$($param),*);
        factory::impl_pipe_factory!(Out;$($param),*);
        factory::impl_into_pipe_factory!(;$($param),*);
        factory::impl_into_pipe_factory!(Out;$($param),*);

        impl_pipe_observer!($($param),*);
        impl_system_pipe!($($param),*);
    };
);

all_tuples!(impl_all, 0, 2, F);

#[cfg(test)]
mod tests {
    use zmaxion_rt::tokio;

    use crate::{
        pipe::factory::{BevyPipeFactory, IntoPipeFactory, PipeFactory},
        prelude::*,
    };

    pub struct A(String);
    pub struct B(String);

    fn transmitter(writer: TopicWriter<A>, reader: TopicReader<B>) {
        writer.write(A("hello".into()));
        for e in read_all!(reader) {
            debug!("{:#?}", e.0);
        }
    }

    async fn async_transmitter(writer: TopicWriter<'static, A>, reader: TopicReader<'static, B>) {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        writer.write(A("hello".into()));
        for e in read_all!(reader) {
            debug!("{:#?}", e.0);
        }
    }

    fn try_fn() -> AnyResult<()> {
        bail!("Sync pipe can return an error")
    }

    async fn async_try_fn() -> AnyResult<u8> {
        bail!("Async pipe can return an error")
    }

    #[test]
    fn into_pipe_factory_implemented() {
        let _ = IntoPipeFactory::into_pipe_factory(transmitter);
        let _ = IntoPipeFactory::into_pipe_factory(async_transmitter);
        let _ = IntoPipeFactory::into_pipe_factory(try_fn);
        let _ = IntoPipeFactory::into_pipe_factory(async_try_fn);
    }
}
