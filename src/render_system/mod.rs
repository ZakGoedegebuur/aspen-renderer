use std::sync::Arc;

use crate::{
    renderpass::RenderPassCont,
    submit_system::SubmitSystem,
    GraphicsObjects,
};

pub trait RenderSystem {
    fn run(&mut self, graphics_objects: Arc<GraphicsObjects>);
}

pub struct DefaultRenderSystem<SST: SubmitSystem> {
    submit_system: SST,
    render_passes: Vec<Box<dyn RenderPassCont<SharedData = SST::SharedType, CmdBufType = SST::CmdBufType> + Send>>,
}

impl<SST: SubmitSystem> DefaultRenderSystem<SST> {
    pub fn new(
        submit_system: SST,
        render_passes: Vec<Box<dyn RenderPassCont<SharedData = SST::SharedType, CmdBufType = SST::CmdBufType> + Send>>,
    ) -> Self {
        Self {
            submit_system,
            render_passes,
        }
    }
}

impl<SST: SubmitSystem> RenderSystem for DefaultRenderSystem<SST> {
    fn run(&mut self, graphics_objects: Arc<GraphicsObjects>) {
        let (shared, setup_data, mut cmd_buf) = match self.submit_system.setup(graphics_objects.clone()) {
            Ok(val) => val,
            Err(_) => return,
        };

        for pass in self.render_passes.iter_mut() {
            match pass.preprocess(graphics_objects.clone(), shared.clone()) {
                Ok(_) => (),
                Err(_) => return,
            }
        }

        for pass in self.render_passes.iter_mut() {
            match pass.build_commands(graphics_objects.clone(), shared.clone(), &mut cmd_buf) {
                Ok(_) => (),
                Err(_) => return,
            }
        }

        for pass in self.render_passes.iter_mut() {
            pass.postprocess(graphics_objects.clone(), shared.clone());
        }

        self.submit_system
            .submit(graphics_objects.clone(), cmd_buf, setup_data, shared)
    }
}