use core::f32;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        Mutex,
    },
};

use aspen_renderer::{
    canvas::Canvas,
    renderpass::{
        CmdBuffer,
        RenderPass,
    },
    GraphicsObjects,
};
use nalgebra::{
    Isometry3,
    Matrix4,
    Perspective3,
    Point3,
    Rotation3,
    UnitVector3,
    Vector3,
};
use vulkano::{
    buffer::{
        allocator::SubbufferAllocator,
        BufferContents,
    },
    descriptor_set::{
        PersistentDescriptorSet,
        WriteDescriptorSet,
    },
    padded::Padded,
    pipeline::{
        graphics::viewport::Viewport,
        GraphicsPipeline,
        Pipeline,
        PipelineBindPoint,
    },
};

use super::present::SharedInfo;
use crate::IndexedMesh;

pub struct CirclesRenderPass {
    pub elapsed_time: f32,
    pub pass_ubo: Arc<Mutex<SubbufferAllocator>>,
    pub obj_ubo: Arc<Mutex<SubbufferAllocator>>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub meshes: HashMap<&'static str, IndexedMesh>,
    pub canvas: Arc<Canvas>,
}

impl RenderPass for CirclesRenderPass {
    type SharedData = SharedInfo;
    type PreProcessed = ();
    type Output = ();

    fn preprocess(
        &mut self,
        gfx_obj: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
    ) -> Result<Self::PreProcessed, aspen_renderer::renderpass::HaltPolicy> {
        if shared.image_extent != self.canvas.extent() {
            self.canvas.recreate_buffers_exact(
                [shared.image_extent[0], shared.image_extent[1], 1],
                shared.num_frames_in_flight,
                gfx_obj.memory_allocator.clone(),
            )
        }

        Ok(())
    }

    fn build_commands(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        shared: Arc<Self::SharedData>,
        cmd_buffer: &mut Box<CmdBuffer>,
        _: Self::PreProcessed,
    ) -> Result<Self::Output, aspen_renderer::renderpass::HaltPolicy> {
        let elapsed_time = self.elapsed_time * 2.0;

        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOPerObject {
            mat: [f32; 16],
            color_offset: Padded<[f32; 3], 4>,
        }

        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOData {
            per_object: [UBOPerObject; 4],
        }

        #[derive(BufferContents)]
        #[repr(C)]
        struct UBOFrameData {
            view: [f32; 16],
            proj: [f32; 16],
        }

        let aspect_ratio = shared.image_extent[0] as f32 / shared.image_extent[1] as f32;

        let view = Isometry3::look_at_rh(
            &Point3::new(1.5, 0.0, -9.0),
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, -1.0, 0.0),
        );

        let proj = Perspective3::new(aspect_ratio, std::f32::consts::FRAC_PI_2, 0.01, 100.0);

        let pass_data = UBOFrameData {
            view: view.to_homogeneous().as_slice().try_into().unwrap(),
            proj: proj.to_homogeneous().as_slice().try_into().unwrap(),
        };

        let data = UBOData {
            per_object: [
                UBOPerObject {
                    mat: {
                        let mut mat = Matrix4::new_scaling(3.0);
                        mat = mat.append_translation(&Vector3::new(
                            0.0 - (elapsed_time + (3.141 * 0.0)).sin() * 5.0,
                            -0.0,
                            0.0,
                        ));
                        let rotation = Rotation3::from_axis_angle(
                            &UnitVector3::new_normalize(-Vector3::z()),
                            elapsed_time % (f32::consts::PI * 2.0),
                        );
                        (mat * rotation.to_homogeneous())
                            .as_slice()
                            .try_into()
                            .unwrap()
                    },
                    color_offset: Padded([0.3, 1.0, 0.5]),
                },
                UBOPerObject {
                    mat: {
                        let mut mat = Matrix4::new_scaling(3.0);
                        mat = mat.append_translation(&Vector3::new(
                            0.0,
                            0.0 + (elapsed_time + (3.141 * 0.25)).sin() * 5.0,
                            3.0,
                        ));
                        let rotation = Rotation3::from_axis_angle(
                            &UnitVector3::new_normalize(-Vector3::z()),
                            elapsed_time % (f32::consts::PI * 2.0),
                        );
                        (mat * rotation.to_homogeneous())
                            .as_slice()
                            .try_into()
                            .unwrap()
                    },
                    color_offset: Padded([1.0, 0.2, 0.5]),
                },
                UBOPerObject {
                    mat: {
                        let mut mat = Matrix4::new_scaling(3.0);
                        mat = mat.append_translation(&Vector3::new(
                            0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 5.0,
                            0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 5.0,
                            6.0,
                        ));
                        let rotation = Rotation3::from_axis_angle(
                            &UnitVector3::new_normalize(-Vector3::z()),
                            elapsed_time % (f32::consts::PI * 2.0),
                        );
                        (mat * rotation.to_homogeneous())
                            .as_slice()
                            .try_into()
                            .unwrap()
                    },
                    color_offset: Padded([0.3, 0.5, 1.0]),
                },
                UBOPerObject {
                    mat: {
                        let mut mat = Matrix4::new_scaling(3.0);
                        mat = mat.append_translation(&Vector3::new(
                            0.0 + (elapsed_time + (3.141 * 0.75)).sin() * 5.0,
                            0.0 - (elapsed_time + (3.141 * 0.75)).sin() * 5.0,
                            9.0,
                        ));
                        let rotation = Rotation3::from_axis_angle(
                            &UnitVector3::new_normalize(-Vector3::z()),
                            elapsed_time % (f32::consts::PI * 2.0),
                        );
                        (mat * rotation.to_homogeneous())
                            .as_slice()
                            .try_into()
                            .unwrap()
                    },
                    color_offset: Padded([1.0, 0.5, 0.2]),
                },
            ],
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
            [WriteDescriptorSet::buffer(0, subbuffer)],
            [],
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
            [WriteDescriptorSet::buffer(0, subbuffer)],
            [],
        )
        .unwrap();

        let mesh = self.meshes.get("hex").unwrap();

        let mut pass_controller = self.canvas.pass_controller();

        pass_controller
            .begin_renderpass(cmd_buffer, [Some([0.2; 3].into()), Some(1.0.into())].into())
            .unwrap();

        cmd_buffer
            .set_viewport(
                0,
                [Viewport {
                    offset: [0.0, 0.0],
                    extent: {
                        //let extent = window.images[shared.image_index as usize].extent();
                        [shared.image_extent[0] as f32, shared.image_extent[1] as f32]
                    },
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                1,
                pass_set.clone(),
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
                object_set.clone(),
            )
            .unwrap()
            .draw_indexed(mesh.ibo.len() as u32, 4, 0, 0, 0)
            .unwrap();
        //.end_render_pass(Default::default())
        //.unwrap();

        pass_controller.end_renderpass(cmd_buffer).unwrap();

        Ok(())
    }

    fn postprocess(&mut self, _: Arc<GraphicsObjects>, _: Arc<Self::SharedData>, _: Self::Output) {}
}
