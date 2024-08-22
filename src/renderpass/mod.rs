use std::sync::Arc;

use vulkano::command_buffer::{
    allocator::StandardCommandBufferAllocator,
    AutoCommandBufferBuilder,
    PrimaryAutoCommandBuffer,
};

use crate::GraphicsObjects;

pub type CmdBuffer = AutoCommandBufferBuilder<
    PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
    Arc<StandardCommandBufferAllocator>,
>;

pub enum HaltPolicy {
    HaltThis,
    HaltAll,
}

pub trait RenderPass {
    type SharedData;
    type PreProcessed;
    type Output;
    type CmdBufType;
    fn preprocess(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
    ) -> Result<Self::PreProcessed, HaltPolicy>;
    fn build_commands(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
        cmd_buffer: &mut Self::CmdBufType,
        preprocessed: Self::PreProcessed,
    ) -> Result<Self::Output, HaltPolicy>;
    fn postprocess(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
        output: Self::Output,
    );
}

pub trait RenderPassCont {
    type SharedData;
    type CmdBufType;
    fn preprocess(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
    ) -> Result<(), HaltPolicy>;
    fn build_commands(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
        cmd_buffer: &mut Self::CmdBufType,
    ) -> Result<(), HaltPolicy>;
    fn postprocess(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
    );
}

enum RenderPassType<PreT, PostT> {
    None,
    PreProcessed(PreT),
    PostProcessed(PostT),
}

pub struct DynamicRenderPass<T: RenderPass> {
    data: RenderPassType<T::PreProcessed, T::Output>,
    inner: T,
}

impl<T: RenderPass> DynamicRenderPass<T> {
    pub fn from_renderpass(renderpass: T) -> Box<Self> {
        Box::new(Self {
            data: RenderPassType::None,
            inner: renderpass,
        })
    }
}

impl<T> From<T> for Box<dyn RenderPassCont<SharedData = T::SharedData, CmdBufType = T::CmdBufType> + Send>
where
    T: RenderPass + Send + 'static,
    T::PreProcessed: Send,
    T::Output: Send,
{
    fn from(value: T) -> Self {
        Box::new(DynamicRenderPass {
            inner: value,
            data: RenderPassType::None,
        })
    }
}

impl<T: RenderPass> RenderPassCont for DynamicRenderPass<T> {
    type SharedData = T::SharedData;
    type CmdBufType = T::CmdBufType;

    fn preprocess(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
    ) -> Result<(), HaltPolicy> {
        self.data = RenderPassType::PreProcessed(self.inner.preprocess(graphics_objects, shared)?);
        Ok(())
    }

    fn build_commands(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
        cmd_buffer: &mut Self::CmdBufType,
    ) -> Result<(), HaltPolicy> {
        let data = match std::mem::replace(&mut self.data, RenderPassType::None) {
            RenderPassType::PreProcessed(data) => data,
            _ => panic!("data not preprocessed"),
        };

        self.data = RenderPassType::PostProcessed(self.inner.build_commands(
            graphics_objects,
            shared,
            cmd_buffer,
            data,
        )?);
        Ok(())
    }

    fn postprocess(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
    ) {
        let data = match std::mem::replace(&mut self.data, RenderPassType::None) {
            RenderPassType::PostProcessed(data) => data,
            _ => panic!("data not postprocessed"),
        };

        self.inner.postprocess(graphics_objects, shared, data);
    }
}
