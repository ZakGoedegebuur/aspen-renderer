use std::sync::Arc;

use aspen_renderer::{
    canvas::Canvas,
    renderpass::RenderPass,
};
use vulkano::{
    command_buffer::BlitImageInfo,
    image::sampler::Filter,
};

use super::present::SharedInfo;

pub struct WindowBlitRenderPass {
    pub src_canvas: Arc<Canvas>,
    pub attachment_index: usize,
}

impl RenderPass for WindowBlitRenderPass {
    type SharedData = SharedInfo;
    type PreProcessed = ();
    type Output = ();

    fn preprocess(
        &mut self,
        _: std::sync::Arc<aspen_renderer::GraphicsObjects>,
        _: std::sync::Arc<Self::SharedData>,
    ) -> Result<Self::PreProcessed, aspen_renderer::renderpass::HaltPolicy> {
        Ok(())
    }

    fn build_commands(
        &mut self,
        _: std::sync::Arc<aspen_renderer::GraphicsObjects>,
        shared: std::sync::Arc<Self::SharedData>,
        cmd_buffer: &mut Box<aspen_renderer::renderpass::CmdBuffer>,
        _: Self::PreProcessed,
    ) -> Result<Self::Output, aspen_renderer::renderpass::HaltPolicy> {
        cmd_buffer
            .blit_image({
                let mut blit = BlitImageInfo::images(
                    self.src_canvas.current_image_set()[self.attachment_index]
                        .image()
                        .clone(),
                    shared.window.lock().unwrap().images[shared.image_index].clone(),
                );

                blit.filter = Filter::Linear;

                blit
            })
            .unwrap();

        Ok(())
    }

    fn postprocess(
        &mut self,
        _: std::sync::Arc<aspen_renderer::GraphicsObjects>,
        _: std::sync::Arc<Self::SharedData>,
        _: Self::Output,
    ) {
    }
}
