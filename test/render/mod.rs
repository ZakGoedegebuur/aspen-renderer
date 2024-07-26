use std::sync::{
    Arc, 
    Mutex
};

use aspen_renderer::{
    window_surface::WindowSurface, 
    GraphicsObjects, 
    RenderScript
};

use vulkano::{
    buffer::BufferContents, command_buffer::{
        AutoCommandBufferBuilder, 
        CommandBufferUsage, 
        RenderPassBeginInfo
    }, descriptor_set::{
        PersistentDescriptorSet, 
        WriteDescriptorSet
    }, pipeline::{
        graphics::vertex_input::Vertex, Pipeline, PipelineBindPoint
    }, swapchain::{
        acquire_next_image, 
        SwapchainCreateInfo, 
        SwapchainPresentInfo
    }, sync::GpuFuture, Validated, VulkanError
};

use crate::RenderData;

#[derive(Debug, BufferContents, Vertex)]
#[repr(C)]
pub struct PosColVertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    pub color: [f32; 3],
}

pub fn circles_renderscript(window: Arc<Mutex<WindowSurface>>, render_data: RenderData) -> RenderScript {
    RenderScript::new(move |GraphicsObjects { device, descriptor_set_allocator, command_buffer_allocator, graphics_queue, .. }| {
        let mut window = window.lock().unwrap();
        let image_extent: [u32; 2] = window.window.inner_size().into();
        
        if image_extent.contains(&0) {
            return
        }

        let previous_frame_index = window.previous_frame_index as usize;
        match window.previous_frame_fences[previous_frame_index].as_mut() {
            Some(f) => f.cleanup_finished(),
            None => ()
        }

        if window.recreate_swapchain {
            let image_extent: [u32; 2] = window.window.inner_size().into();
            let (new_swapchain, new_images) = window.swapchain
            .recreate(SwapchainCreateInfo {
                image_extent,
                ..window.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");

            window.swapchain = new_swapchain;
            
            window.images = new_images;
            let render_pass = window.framebuffers[0].render_pass().clone();
            window.image_size_dependent_setup(render_pass);
            window.recreate_swapchain = false;
        }

        let (image_index, suboptimal, acquire_future) = 
            match acquire_next_image(window.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    window.recreate_swapchain = true;
                    return
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            window.recreate_swapchain = true;
        }

        let elapsed_time = render_data.elapsed_time * 2.0;
        let v_offset = [0.0, 0.0];
        let aspect_ratio = image_extent[1] as f32 / image_extent[0] as f32;
        
        let data: [f32; 40] = [
            aspect_ratio, 0.5,
            v_offset[0], v_offset[1],
            0.0, 0.0,
            0.0, 0.0,
            
            0.0 - (elapsed_time + (3.141 * 0.0)).sin() * 1.5, -0.0,
            0.45, 0.45,
            0.3, 1.0, 0.5, 1.0,
            
            0.0, 0.0 + (elapsed_time + (3.141 * 0.25)).sin() * 1.5,
            0.45, 0.45,
            1.0, 0.2, 0.5, 1.0,
            
            0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 1.5, 0.0 + (elapsed_time + (3.141 * 0.5)).sin() * 1.5,
            0.45, 0.45,
            0.3, 0.5, 1.0, 1.0,
            
            0.0 + (elapsed_time + (3.141 * 0.75)).sin() * 1.5, 0.0 - (elapsed_time + (3.141 * 0.75)).sin() * 1.5,
            0.45, 0.45,
            1.0, 0.5, 0.2, 1.0,
        ];

        let subbuffer = {
            let ubo = render_data.ubo.lock().unwrap();
            let subbuffer = ubo.allocate_sized().unwrap();
            *subbuffer.write().unwrap() = data;
            subbuffer
        };
        
        let set = PersistentDescriptorSet::new(
            descriptor_set_allocator, 
            render_data.pipeline.layout().set_layouts()[0].clone(), 
                [
                    WriteDescriptorSet::buffer(0, subbuffer)
                ], 
                []
            )
            .unwrap();
        
        let mut builder = AutoCommandBufferBuilder::primary(
            command_buffer_allocator, 
            graphics_queue.queue_family_index(), 
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        let mesh = render_data.meshes.get("hex").unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.07, 0.07, 0.07, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer( 
                        window.framebuffers[image_index as usize].clone(),
                    )
                },
                Default::default(),
            )
            .unwrap()
            .set_viewport(0, [window.viewport.clone()].into_iter().collect())
            .unwrap()
            .bind_pipeline_graphics(render_data.pipeline.clone())
            .unwrap()
            .bind_vertex_buffers(0, mesh.vbo.clone())
            .unwrap()
            .bind_index_buffer(mesh.ibo.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                render_data.pipeline.layout().clone(), 
                0, 
                set.clone()
            )
            .unwrap()
            .draw_indexed(mesh.ibo.len() as u32, 4, 0, 0, 0)
            .unwrap()
            .end_render_pass(Default::default())
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let previous_future = match window.previous_frame_fences[image_index as usize].clone() {
            None => {
                let mut now = vulkano::sync::now(device.clone());
                now.cleanup_finished();

                now.boxed_send()
            }
            Some(mut fence) => {
                fence.cleanup_finished();
                fence.boxed_send()
            }
        };

        let future = previous_future
            .join(acquire_future)
            .then_execute(graphics_queue.clone(), command_buffer.clone())
            .unwrap()
            .then_swapchain_present(
                graphics_queue.clone(), 
                SwapchainPresentInfo::swapchain_image_index(window.swapchain.clone(), image_index)
            )
            .boxed_send()
            .then_signal_fence_and_flush();

        window.previous_frame_fences[image_index as usize] = match future.map_err(Validated::unwrap) {
            Ok(value) => Some(Arc::new(value)),
            Err(VulkanError::OutOfDate) => {
                window.recreate_swapchain = true;
                let winextent = window.window.inner_size();
                let swapextent: Vec<[u32; 3]> = window.images.iter().map(|image| image.extent()).collect();
                println!("Fence out of date.\nWindow size:\n{:#?}\nSwapchain image sizes:\n{:#?}", winextent, swapextent);
                None
            },
            Err(e) => {
                println!("failed to flush future: {e}");
                None
            }
        };

        window.previous_frame_index = image_index;
    })
}