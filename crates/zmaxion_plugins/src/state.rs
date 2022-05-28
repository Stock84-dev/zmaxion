use zmaxion_app::prelude::*;
use zmaxion_core::state::components::PipelineState;

use crate::prelude::*;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build<'a, 'b>(self: Box<Self>, builder: &'b mut AppBuilder<'a>) -> &'b mut AppBuilder<'a> {
        builder.register_topic::<PipelineState>()
    }
}
