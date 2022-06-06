use std::{future::Future, pin::Pin};

use bevy_ecs::{all_tuples, system::SystemParamState};
use paste::paste;

use crate::{PipeParamState, PipeParamStateImpl, *};
macro_rules! impl_pipe_param (
    ($($param: ident),*) => {
        impl<$($param: PipeParam,)*> PipeParam for ($($param,)*) {
            type State = ($(<$param as PipeParam>::State,)*);
        }
    };
);

all_tuples!(impl_pipe_param, 0, 16, F);

macro_rules! impl_pipe_param_fetch (
    ($($state: ident),*) => {
        impl<'s, $($state: PipeParamFetch<'s>),*> PipeParamFetch<'s> for ($($state,)*) {
            type Item = ($(<$state as PipeParamFetch<'s>>::Item,)*);

            fn get_param(&'s mut self) -> Self::Item {
                let ($($state,)*) = self;
                ($($state::get_param($state),)*)
            }
        }
    };
);

all_tuples!(impl_pipe_param_fetch, 0, 16, F);

macro_rules! impl_pipe_observer (
    ($($param: ident),*) => {
        impl<$($param: PipeObserver,)*> PipeObserver for ($($param,)*) {
            fn pipe_executing(&mut self) {
                let ($($param,)*) = self;
                $($param.pipe_executing();)*
            }
            fn pipe_executed<T>(&mut self, result: &AnyResult<T>) -> ControlFlow {
                let mut flow = ControlFlow::default();
                let ($($param,)*) = self;
                $(
                    flow |= $param.pipe_executed(result);
                )*
                flow
            }
        }
    };
);

all_tuples!(impl_pipe_observer, 0, 16, F);

fn build_param<P: PipeParam>(
    defined_kind: TopicParamKind,
    args: &PipeFactoryArgs,
    pipe_args: &mut &[u8],
    arg_i: &mut usize,
    writer_i: &mut usize,
    reader_i: &mut usize,
    type_name: &'static str,
) -> AnyResult<Pin<Box<dyn Future<Output = AnyResult<P::State>> + Send>>>
where
    P::State: Send,
    <P as PipeParam>::State: PipeParamState + 'static,
{
    let topic_param =
        match defined_kind {
            TopicParamKind::Reader => {
                let id = args.reader_topics.get(*reader_i).ok_or_else(|| {
                    PipeParamError::InvalidPipeParam(*arg_i, type_name.to_string())
                })?;
                *reader_i += 1;
                Some(TopicParam::Reader(*id))
            }
            TopicParamKind::Writer => {
                let id = args.writer_topics.get(*writer_i).ok_or_else(|| {
                    PipeParamError::InvalidPipeParam(*arg_i, type_name.to_string())
                })?;
                *writer_i += 1;
                Some(TopicParam::Writer(*id))
            }
            TopicParamKind::None => None,
        };
    let builder = args.to_param_builder(*arg_i, topic_param, pipe_args, defined_kind);
    *arg_i += 1;
    Ok(<<P as PipeParam>::State as PipeParamState>::new(builder))
}

macro_rules! impl_build_params {
    ($($param:ident),*) => {
        paste! {
            pub struct [< P$( $param)* >];
        }
        paste! {
            #[allow(non_snake_case)]
            impl [< P$( $param)* >] {
                pub async fn build_params<$($param),*>(args: PipeFactoryArgs) -> AnyResult<($(<$param as PipeParam>::State,)*), PipeParamError>
                where
                    $(
                        $param: PipeParam + 'static,
                        <$param as PipeParam>::State: PipeParamState + Send,
                    )*
                {
                    let mut arg_i = 0;
                    let mut reader_i = 0;
                    let mut writer_i = 0;
                    #[allow(unused_mut)]
                    let mut pipe_args = &args.config.config.args[..];

                    let result = futures_util::try_join!(
                        $({
                            build_param::<$param>(
                                <$param::State as PipeParamState>::KIND,
                                &args,
                                &mut pipe_args,
                                &mut arg_i,
                                &mut writer_i,
                                &mut reader_i,
                                $param::type_name()
                            )?
                        }),*
                    );
                    result.map_err(|e| PipeParamError::Other(e))
                }
            }
        }
    };
}

all_tuples!(impl_build_params, 0, 16, F);

impl<P: PipeParamImpl> PipeParam for P {
    type State = P::State;
}

impl<S: PipeParamStateImpl<'static> + Send> PipeParamState for S {
    type Args = S::Args;

    const KIND: TopicParamKind = TopicParamKind::None;

    fn new<'a>(
        builder: ParamBuilder<Self::Args>,
    ) -> Pin<Box<dyn Future<Output = AnyResult<Self>> + Send + 'a>>
    where
        Self: Sized,
    {
        S::new(builder)
    }

    fn topic_param(&self) -> Option<TopicParam> {
        S::topic_param(self)
    }
}

impl<S: PipeParamStateImpl<'static>> PipeObserver for S {
    fn pipe_executing(&mut self) {
        S::pipe_executing(self)
    }

    fn pipe_executed<T>(&mut self, result: &AnyResult<T>) -> ControlFlow {
        S::pipe_executed(self, result)
    }
}

impl<'s, S: PipeParamStateImpl<'s>> PipeParamFetch<'s> for S {
    type Item = S::Param;

    fn get_param(&'s mut self) -> Self::Item {
        S::get_param(self)
    }
}
