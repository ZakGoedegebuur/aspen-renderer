use std::sync::Arc;

use crate::{
    renderpass::RenderPassCont, 
    submit_system::SubmitSystemCont, 
    GraphicsObjects
};

pub trait RenderSystem {
    fn run(&mut self, graphics_objects: Arc<GraphicsObjects>);
}

pub struct DefaultRenderSystem<SST: SubmitSystemCont> {
    submit_system: SST,
    render_passes: Vec<Box<dyn RenderPassCont<SharedData = SST::SharedType> + Send>>
}

impl<SST: SubmitSystemCont> DefaultRenderSystem<SST> {
    pub fn new(
        submit_system: SST,
        render_passes: Vec<Box<dyn RenderPassCont<SharedData = SST::SharedType> + Send>>
    ) -> Self {
        Self {
            submit_system,
            render_passes: render_passes.into_iter().collect()
        }
    }
}

impl<SST: SubmitSystemCont> RenderSystem for DefaultRenderSystem<SST> {
    fn run(&mut self, graphics_objects: Arc<GraphicsObjects>) {
        let (shared, mut cmd_buf) = match self.submit_system.setup(graphics_objects.clone()) {
            Ok(val) => val,
            Err(_) => return
        };

        let shared = Arc::new(shared);

        for pass in self.render_passes.iter_mut() {
            match pass.preprocess(graphics_objects.clone(), shared.clone()) {
                Ok(_) => (),
                Err(_) => return
            }
        }

        for pass in self.render_passes.iter_mut() {
            match pass.build_commands(graphics_objects.clone(), shared.clone(), &mut cmd_buf) {
                Ok(_) => (),
                Err(_) => return
            }
        }

        for pass in self.render_passes.iter_mut() {
            pass.postprocess(graphics_objects.clone(), shared.clone());
        }

        self.submit_system.submit(graphics_objects.clone(), cmd_buf)
    }
}