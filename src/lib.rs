pub mod canvas;
pub mod drawable;
pub mod render_system;
pub mod renderpass;
pub mod submit_system;
pub mod window_surface;

use std::{
    collections::HashMap,
    sync::{
        mpsc::{
            channel,
            sync_channel,
            Receiver,
            Sender,
            SyncSender,
        },
        Arc,
    },
    thread,
};

use parking_lot::Mutex;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{
        physical::PhysicalDeviceType,
        Device,
        DeviceCreateInfo,
        DeviceExtensions,
        Features,
        Queue,
        QueueCreateInfo,
        QueueFlags,
    },
    image::{
        view::ImageView,
        Image,
        ImageUsage,
    },
    instance::{
        Instance,
        InstanceCreateFlags,
        InstanceCreateInfo,
    },
    memory::allocator::StandardMemoryAllocator,
    pipeline::graphics::viewport::Viewport,
    render_pass::{
        Framebuffer,
        FramebufferCreateInfo,
        RenderPass,
    },
    swapchain::{
        Surface,
        Swapchain,
        SwapchainCreateInfo,
    },
    VulkanLibrary,
};
use window_surface::WindowSurface;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{
        WindowBuilder,
        WindowId,
    },
};

use crate::render_system::RenderSystem;

#[derive(Clone)]
pub struct GraphicsObjects {
    pub num_frames_in_flight: usize,
    pub device: Arc<Device>,
    pub graphics_queue: Arc<Queue>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
}

pub struct Renderer {
    pub comms: RenderThreadComms,
    pub windows: HashMap<WindowId, Arc<Mutex<WindowSurface>>>,
    pub graphics_objects: GraphicsObjects,
}

impl Renderer {
    pub fn new<ELT>(event_loop: &EventLoop<ELT>) -> (Self, WindowId) {
        let library = VulkanLibrary::new().unwrap();
        let required_extensions = Surface::required_extensions(&event_loop);

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .unwrap();

        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Primary window")
                .with_inner_size(PhysicalSize::new(400, 400))
                .build(&event_loop)
                .unwrap(),
        );

        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .expect("no suitable physical device found");

        println!(
            "Using device: {} (type: {:?})\nVulkan version: {}\nCompute subgroup size: {}\nVertex buffer binding limit: {}",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
            physical_device.api_version(),
            physical_device.properties().subgroup_size.unwrap(),
            physical_device.properties().max_vertex_input_bindings,
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: Features {
                    fill_mode_non_solid: true,
                    ..Default::default()
                },
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],

                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let surface_capabilities = device
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let num_frames_in_flight = surface_capabilities.min_image_count.max(2);

        let surface_image_format = device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

        let (swapchain, images) = {
            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: num_frames_in_flight,
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

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        //let framebuffers = window_size_dependent_setup(&images, renderpass.clone(), &mut viewport);
        let previous_frame_fences = (0..images.len()).map(|_| None).collect::<Vec<_>>();

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let mut windows = HashMap::new();
        let window_id = window.id();
        windows.insert(
            window_id,
            Arc::new(Mutex::new(WindowSurface {
                window,
                swapchain,
                images,
                framebuffers: Vec::new(),
                //viewport,
                recreate_swapchain: true,
                previous_frame_fences,
                num_frames_in_flight: 0,
                previous_frame_index: 0,
                surface_image_format,
            })),
        );

        //let window_2 = WindowSurface::new(event_loop, device.clone());
        //windows.insert(window_2.window.id(), Arc::new(Mutex::new(window_2)));

        let graphics_objects_original = GraphicsObjects {
            num_frames_in_flight: num_frames_in_flight as usize,
            device: device.clone(),
            graphics_queue: queue.clone(),
            descriptor_set_allocator: descriptor_set_allocator.clone(),
            command_buffer_allocator: command_buffer_allocator.clone(),
            memory_allocator: memory_allocator.clone(),
        };

        let graphics_objects = graphics_objects_original.clone();

        let (sender, reciever) = sync_channel::<(Box<dyn RenderSystem + Send>, Sender<()>)>(1);
        let render_closure = move || {
            let graphics_objects = Arc::new(graphics_objects_original.clone());
            loop {
                match reciever.recv() {
                    Err(_) => break,
                    Ok((mut rendergraph, msender)) => {
                        rendergraph.run(graphics_objects.clone());

                        _ = msender.send(())
                    }
                }
            }
        };

        let render_thread = thread::Builder::new()
            .name("main_render_thread".to_string())
            .spawn(render_closure)
            .expect("failed to spawn main render thread");

        let comms = RenderThreadComms {
            sender: Some(sender),
            render_thread: Some(render_thread),
        };

        (
            Self {
                comms,
                windows,
                graphics_objects,
            },
            window_id,
        )
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.graphics_objects.device
    }

    pub fn allocator(&self) -> &Arc<StandardMemoryAllocator> {
        &self.graphics_objects.memory_allocator
    }
}

pub struct RenderThreadComms {
    pub sender: Option<SyncSender<(Box<dyn RenderSystem + Send>, Sender<()>)>>,
    pub render_thread: Option<thread::JoinHandle<()>>,
}

impl RenderThreadComms {
    pub fn send(&mut self, render_system: impl RenderSystem + Send + 'static) -> PresentBarrier {
        let (sender, reciever) = channel();
        self.sender
            .as_ref()
            .unwrap()
            .send((Box::new(render_system), sender))
            .expect("Render thread hung up");
        PresentBarrier {
            reciever: Some(reciever),
        }
    }
}

impl Drop for RenderThreadComms {
    fn drop(&mut self) {
        _ = self.sender.take();
        _ = self.render_thread.take().unwrap().join();
    }
}

pub struct PresentBarrier {
    reciever: Option<Receiver<()>>,
}

impl PresentBarrier {
    pub fn blocking_wait(&mut self) {
        if let Some(reciever) = self.reciever.as_ref() {
            _ = reciever.recv();
            self.reciever = None
        }
    }
}

impl Drop for PresentBarrier {
    fn drop(&mut self) {
        self.blocking_wait()
    }
}

fn window_size_dependent_setup(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}