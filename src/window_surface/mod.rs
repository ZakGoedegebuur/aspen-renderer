use std::sync::Arc;

use vulkano::{
    device::Device,
    format::Format,
    image::{
        Image,
        ImageUsage,
    },
    render_pass::Framebuffer,
    swapchain::{
        Surface,
        Swapchain,
        SwapchainCreateInfo,
    },
    sync::{
        future::FenceSignalFuture,
        GpuFuture,
    },
};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{
        Window,
        WindowBuilder,
    },
};

pub struct WindowSurface {
    pub window: Arc<Window>,
    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<Image>>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub recreate_swapchain: bool,
    pub previous_frame_fences: Vec<Option<Arc<FenceSignalFuture<Box<dyn GpuFuture + Send>>>>>,
    pub num_frames_in_flight: usize,
    pub previous_frame_index: usize,
    pub surface_image_format: Format,
}

impl WindowSurface {
    pub fn new<ELT>(event_loop: &EventLoop<ELT>, device: Arc<Device>) -> Self {
        let window = WindowBuilder::new()
            .with_title("New window")
            .with_inner_size(PhysicalSize::new(400, 400))
            .build(event_loop)
            .unwrap();

        let window = Arc::new(window);

        let surface = Surface::from_window(device.instance().clone(), window.clone()).unwrap();

        let surface_image_format = device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

        let (swapchain, images) = {
            let surface_capabilities = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format: surface_image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .next()
                        .unwrap(),
                    present_mode: vulkano::swapchain::PresentMode::Fifo,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let previous_frame_fences = (0..images.len()).map(|_| None).collect::<Vec<_>>();

        Self {
            window,
            swapchain,
            images,
            framebuffers: Vec::new(),
            previous_frame_fences,
            recreate_swapchain: true,
            num_frames_in_flight: 0,
            previous_frame_index: 0,
            surface_image_format,
        }
    }
}
