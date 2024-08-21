use std::sync::{
    Arc,
    Mutex,
};

use aspen_renderer::{
    renderpass::{
        CmdBuffer,
        HaltPolicy,
    },
    submit_system::SubmitSystem,
    window_surface::WindowSurface,
    GraphicsObjects,
};
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder,
        CommandBufferUsage,
    },
    swapchain::{
        acquire_next_image,
        SwapchainAcquireFuture,
        SwapchainCreateInfo,
        SwapchainPresentInfo,
    },
    sync::GpuFuture,
    Validated,
    VulkanError,
};

pub struct PresentSystem {
    pub window: Arc<Mutex<WindowSurface>>,
    //pub renderpass: Arc<RenderPass>
}

pub struct SharedInfo {
    pub window: Arc<Mutex<WindowSurface>>,
    pub num_frames_in_flight: usize,
    pub image_index: usize,
    pub image_extent: [u32; 2],
}

pub struct SetupData {
    pub acquire_future: SwapchainAcquireFuture,
}

impl SubmitSystem for PresentSystem {
    type SharedType = SharedInfo;
    type SetupType = SetupData;

    fn setup(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
    ) -> Result<(Arc<Self::SharedType>, Self::SetupType, Box<CmdBuffer>), HaltPolicy> {
        let mut window = self.window.lock().unwrap();
        let image_extent: [u32; 2] = window.window.inner_size().into();

        if image_extent.contains(&0) {
            return Err(HaltPolicy::HaltAll);
        }

        let previous_frame_index = window.previous_frame_index as usize;
        match window.previous_frame_fences[previous_frame_index].as_mut() {
            Some(f) => f.cleanup_finished(),
            None => (),
        }

        if window.recreate_swapchain {
            let image_extent: [u32; 2] = window.window.inner_size().into();
            let (new_swapchain, new_images) = window
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent,
                    ..window.swapchain.create_info()
                })
                .expect("failed to recreate swapchain");

            window.swapchain = new_swapchain;
            window.images = new_images;
            window.num_frames_in_flight = window.images.len();
            //let render_pass = self.renderpass.clone();
            //window.image_size_dependent_setup(render_pass);
            window.recreate_swapchain = false;
        }

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(window.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    window.recreate_swapchain = true;
                    return Err(HaltPolicy::HaltAll);
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            window.recreate_swapchain = true;
        }

        let builder = Box::new(
            AutoCommandBufferBuilder::primary(
                &graphics_objects.command_buffer_allocator,
                graphics_objects.graphics_queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap(),
        );

        Ok((
            Arc::new(SharedInfo {
                window: self.window.clone(),
                num_frames_in_flight: window.num_frames_in_flight,
                image_index: image_index as usize,
                image_extent: [image_extent[0], image_extent[1]],
            }),
            SetupData { acquire_future },
            builder,
        ))
    }

    fn submit(
        &mut self,
        graphics_objects: Arc<GraphicsObjects>,
        cmd_buffer: Box<CmdBuffer>,
        setup_data: Self::SetupType,
        shared: Arc<Self::SharedType>,
    ) {
        let mut window = self.window.lock().unwrap();

        let command_buffer = cmd_buffer.build().unwrap();

        let previous_future = match window.previous_frame_fences[shared.image_index].clone() {
            None => {
                let mut now = vulkano::sync::now(graphics_objects.device.clone());
                now.cleanup_finished();

                now.boxed_send()
            }
            Some(mut fence) => {
                fence.cleanup_finished();
                fence.boxed_send()
            }
        };

        let future = previous_future
            .join(setup_data.acquire_future)
            .then_execute(
                graphics_objects.graphics_queue.clone(),
                command_buffer.clone(),
            )
            .unwrap()
            .then_swapchain_present(
                graphics_objects.graphics_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    window.swapchain.clone(),
                    shared.image_index as u32,
                ),
            )
            .boxed_send()
            .then_signal_fence_and_flush();

        window.previous_frame_fences[shared.image_index] = match future.map_err(Validated::unwrap) {
            Ok(value) => Some(Arc::new(value)),
            Err(VulkanError::OutOfDate) => {
                let winextent = window.window.inner_size();
                let swapextent: Vec<[u32; 3]> =
                    window.images.iter().map(|image| image.extent()).collect();
                println!(
                    "Fence out of date.\nWindow size:\n{:#?}\nSwapchain image sizes:\n{:#?}",
                    winextent, swapextent
                );
                None
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                None
            }
        };

        window.previous_frame_index = shared.image_index;
    }
}
