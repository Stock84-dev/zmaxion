pub mod components {
    use crate::prelude::*;
    use crate::state::resources::PipelineStateData;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[derive(Component, Clone)]
    pub struct PipelineState(Arc<PipelineStateInner>);

    pub struct PipelineStateInner {
        state: PipelineStateData,
        updated: AtomicBool,
    }
}

pub mod resources {
    use crate::pipeline::components::PipeIdRelToPipeline;
    use crate::sync::Mutex;
    use bevy::utils::HashMap;

    pub struct PipelineStateData(pub HashMap<PipeIdRelToPipeline, Mutex<Vec<u8>>>);
    pub struct SerializedPipelineState(Vec<u8>);
}
