use std::{collections::HashMap, sync::{Arc, Mutex}};

use aspen_renderer::{renderpass::{CmdBuffer, RenderPass}, GraphicsObjects};
use vulkano::{
    buffer::{
        allocator::SubbufferAllocator, 
        BufferContents
    }, command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo}, descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet}, padded::Padded, pipeline::{
        GraphicsPipeline, Pipeline, PipelineBindPoint
    }
};

use crate::IndexedMesh;

use super::present::SharedInfo;

pub struct CirclesRenderPass {
    pub elapsed_time: f32,
    pub ubo: Arc<Mutex<SubbufferAllocator>>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub meshes: HashMap<&'static str, IndexedMesh>
}

impl RenderPass for CirclesRenderPass {
    type SharedData = SharedInfo;
    type PreProcessed = ();
    type Output = ();

    fn preprocess(&mut self, _: Arc<GraphicsObjects>, _: Arc<Self::SharedData>) -> Result<Self::PreProcessed, aspen_renderer::renderpass::HaltPolicy> {
        Ok(())
    }

    fn build_commands(&mut self, graphics_objects: Arc<GraphicsObjects>, shared: Arc<Self::SharedData>, cmd_buffer: &mut Box<CmdBuffer>, _: Self::PreProcessed) -> Result<Self::Output, aspen_renderer::renderpass::HaltPolicy> {
        let elapsed_time = self.elapsed_time * 2.0;
        let aspect_ratio = shared.image_extent[1] as f32 / shared.image_extent[0] as f32;
        
        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOHeader {
            aspect_ratio: f32,
            viewport_scale: f32,
            viewport_offset: [f32; 2],
            time: f32,
        }

        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOPerObject {
            offset: [f32; 2],
            scale: [f32; 2],
            color_offset: Padded<[f32; 3], 4>
        }

        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOData {
            header: Padded<UBOHeader, 12>,
            per_object: [UBOPerObject; 4]
        }

        let data = UBOData {
            header: Padded(UBOHeader {
                aspect_ratio,
                viewport_scale: 0.5,
                viewport_offset: [0.0, 0.0],
                time: elapsed_time
            }),
            per_object: [
                UBOPerObject {
                    offset: [0.0 - (elapsed_time + (3.141 * 0.0)).sin() * 1.5, -0.0],
                    scale: [0.45, 0.45],
                    color_offset: Padded([0.3, 1.0, 0.5])
                },
                UBOPerObject {
                    offset: [0.0, 0.0 + (elapsed_time + (3.141 * 0.25)).sin() * 1.5],
                    scale: [0.45, 0.45],
                    color_offset: Padded([1.0, 0.2, 0.5])
                },
                UBOPerObject {
                    offset: [0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 1.5, 0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 1.5],
                    scale: [0.45, 0.45],
                    color_offset: Padded([0.3, 0.5, 1.0])
                },
                UBOPerObject {
                    offset: [0.0 + (elapsed_time + (3.141 * 0.75)).sin() * 1.5, 0.0 - (elapsed_time + (3.141 * 0.75)).sin() * 1.5],
                    scale: [0.45, 0.45],
                    color_offset: Padded([1.0, 0.5, 0.2])
                },
            ]
        };

        let subbuffer = {
            let ubo = self.ubo.lock().unwrap();
            let subbuffer = ubo.allocate_sized().unwrap();
            *subbuffer.write().unwrap() = data;
            subbuffer
        };
        
        let set = PersistentDescriptorSet::new(
                &graphics_objects.descriptor_set_allocator, 
                self.pipeline.layout().set_layouts()[0].clone(), 
                [
                    WriteDescriptorSet::buffer(0, subbuffer)
                ], 
                []
            )
            .unwrap();
        
        let mesh = self.meshes.get("hex").unwrap();

        let window = shared.window.lock().unwrap();

        cmd_buffer
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.07, 0.07, 0.07, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer( 
                        window.framebuffers[shared.current_image_index as usize].clone(),
                    )
                },
                Default::default(),
            )
            .unwrap()
            .set_viewport(0, [window.viewport.clone()].into_iter().collect())
            .unwrap()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .bind_vertex_buffers(0, mesh.vbo.clone())
            .unwrap()
            .bind_index_buffer(mesh.ibo.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.pipeline.layout().clone(), 
                0, 
                set.clone()
            )
            .unwrap()
            .draw_indexed(mesh.ibo.len() as u32, 4, 0, 0, 0)
            .unwrap()
            .end_render_pass(Default::default())
            .unwrap();

        Ok(())
    }

    fn postprocess(&mut self, _: Arc<GraphicsObjects>, _: Arc<Self::SharedData>, _: Self::Output) {
        
    }
}