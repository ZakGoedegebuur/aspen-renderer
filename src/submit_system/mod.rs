use std::sync::Arc;

use crate::{
    renderpass::HaltPolicy,
    GraphicsObjects,
};

pub trait SubmitSystem {
    type SharedType;
    type SetupType;
    type CmdBufType;
    fn setup(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
    ) -> Result<(Arc<Self::SharedType>, Self::SetupType, Self::CmdBufType), HaltPolicy>;
    fn submit(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        cmd_buffer: Self::CmdBufType,
        setup_data: Self::SetupType,
        shared_data: Arc<Self::SharedType>,
    );
}