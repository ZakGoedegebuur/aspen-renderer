use core::f32;
use std::{collections::HashMap, sync::{Arc, Mutex}};

use aspen_renderer::{renderpass::{CmdBuffer, RenderPass}, GraphicsObjects};
use glam::{Mat3, Mat4, Quat, Vec3};
use vulkano::{
    buffer::{
        allocator::SubbufferAllocator, 
        BufferContents
    }, command_buffer::RenderPassBeginInfo, descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet}, padded::Padded, pipeline::{
        GraphicsPipeline, Pipeline, PipelineBindPoint
    }
};

use crate::IndexedMesh;

use super::present::SharedInfo;

pub struct CirclesRenderPass {
    pub elapsed_time: f32,
    pub pass_ubo: Arc<Mutex<SubbufferAllocator>>,
    pub obj_ubo: Arc<Mutex<SubbufferAllocator>>,
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
        
        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOPerObject {
            mat: [[f32; 4]; 4],
            color_offset: Padded<[f32; 3], 4>
        }

        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOData {
            per_object: [UBOPerObject; 4]
        }

        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOFrameData {
            view: [[f32; 4]; 4],
            proj: [[f32; 4]; 4],
        }

        let aspect_ratio = shared.image_extent[0] as f32 / shared.image_extent[1] as f32;

        let proj = Mat4::perspective_rh_gl(
            std::f32::consts::FRAC_PI_2,
            aspect_ratio,
            0.01,
            100.0,
        );
        
        let view = Mat4::look_at_rh(
            Vec3::new(5.0, 5.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        );

        let scale = Mat4::from_scale(Vec3::splat(0.5));

        let pass_data = UBOFrameData {
            view: (view * scale).to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
        };

        let data = UBOData {
            per_object: [
                UBOPerObject {
                    mat: {
                        let translation = Vec3::new(0.0 - (elapsed_time + (3.141 * 0.0)).sin() * 5.0, -0.0, 0.0);
                        let scale = Vec3::splat(1.0);
                        let rotation = Quat::from_rotation_z(elapsed_time % (f32::consts::PI * 2.0));
                        Mat4::from_scale_rotation_translation(scale, rotation, translation).to_cols_array_2d()
                    },
                    color_offset: Padded([0.3, 1.0, 0.5])
                },
                UBOPerObject {
                    mat: {
                        let translation = Vec3::new(0.0, 0.0 + (elapsed_time + (3.141 * 0.25)).sin() * 5.0, 0.0);
                        let scale = Vec3::splat(1.0);
                        let rotation = Quat::from_rotation_z(elapsed_time % (f32::consts::PI * 2.0));
                        Mat4::from_scale_rotation_translation(scale, rotation, translation).to_cols_array_2d()
                    },
                    color_offset: Padded([1.0, 0.2, 0.5])
                },
                UBOPerObject {
                    mat: {
                        let translation = Vec3::new(0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 5.0, 0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 5.0, 0.0);
                        let scale = Vec3::splat(1.0);
                        let rotation = Quat::from_rotation_z(elapsed_time % (f32::consts::PI * 2.0));
                        Mat4::from_scale_rotation_translation(scale, rotation, translation).to_cols_array_2d()
                    },
                    color_offset: Padded([0.3, 0.5, 1.0])
                },
                UBOPerObject {
                    mat: {
                        let translation = Vec3::new(0.0 + (elapsed_time + (3.141 * 0.75)).sin() * 5.0, 0.0 - (elapsed_time + (3.141 * 0.75)).sin() * 5.0, 0.0);
                        let scale = Vec3::splat(1.0);
                        let rotation = Quat::from_rotation_z(elapsed_time % (f32::consts::PI * 2.0));
                        Mat4::from_scale_rotation_translation(scale, rotation, translation).to_cols_array_2d()
                    },
                    color_offset: Padded([1.0, 0.5, 0.2])
                },
            ]
        };

        let subbuffer = {
            let ubo = self.pass_ubo.lock().unwrap();
            let subbuffer = ubo.allocate_sized().unwrap();
            *subbuffer.write().unwrap() = pass_data;
            subbuffer
        };

        let pass_set = PersistentDescriptorSet::new(
            &graphics_objects.descriptor_set_allocator, 
            self.pipeline.layout().set_layouts()[1].clone(), 
            [
                WriteDescriptorSet::buffer(0, subbuffer)
            ], 
            []
        )
        .unwrap();

        let subbuffer = {
            let ubo = self.obj_ubo.lock().unwrap();
            let subbuffer = ubo.allocate_sized().unwrap();
            *subbuffer.write().unwrap() = data;
            subbuffer
        };

        let object_set = PersistentDescriptorSet::new(
            &graphics_objects.descriptor_set_allocator, 
            self.pipeline.layout().set_layouts()[3].clone(), 
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
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.pipeline.layout().clone(), 
                1, 
                pass_set.clone()
            )
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
                3, 
                object_set.clone()
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