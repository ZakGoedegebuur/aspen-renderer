use std::{
    collections::HashMap,
    io::Read,
    sync::{
        Arc,
        Mutex,
    },
    time::Instant,
};

use aspen_renderer::{
    canvas::Canvas,
    render_system::DefaultRenderSystem,
    Renderer,
};
use passes::{
    circles::CirclesRenderPass,
    present::PresentSystem,
    window_blit::WindowBlitRenderPass,
};
use vulkano::{
    buffer::{
        allocator::{
            SubbufferAllocator,
            SubbufferAllocatorCreateInfo,
        },
        Buffer,
        BufferContents,
        BufferCreateInfo,
        BufferUsage,
        Subbuffer,
    },
    descriptor_set::layout::{
        DescriptorSetLayoutBinding,
        DescriptorSetLayoutCreateFlags,
        DescriptorSetLayoutCreateInfo,
        DescriptorType,
    },
    format::Format,
    image::{
        ImageCreateInfo,
        ImageType,
        ImageUsage,
    },
    memory::allocator::{
        AllocationCreateInfo,
        MemoryTypeFilter,
    },
    pipeline::{
        graphics::{
            color_blend::{
                ColorBlendAttachmentState,
                ColorBlendState,
            },
            depth_stencil::{
                DepthState,
                DepthStencilState,
            },
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::{
                CullMode,
                FrontFace,
                RasterizationState,
            },
            vertex_input::{
                Vertex,
                VertexInputAttributeDescription,
                VertexInputBindingDescription,
                VertexInputState,
            },
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::{
            PipelineDescriptorSetLayoutCreateInfo,
            PipelineLayoutCreateFlags,
        },
        DynamicState,
        GraphicsPipeline,
        PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
    shader::{
        ShaderModule,
        ShaderModuleCreateInfo,
        ShaderStages,
    },
};
use winit::{
    event::{
        Event,
        WindowEvent,
    },
    event_loop::{
        ControlFlow,
        EventLoopBuilder,
    },
};

mod passes;

pub struct RenderData {
    pub elapsed_time: f32,
    pub ubo: Arc<Mutex<SubbufferAllocator>>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub meshes: HashMap<&'static str, IndexedMesh>,
}

#[derive(Debug, BufferContents, Vertex)]
#[repr(C)]
pub struct PosColVertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    pub color: [f32; 3],
}

#[derive(Clone)]
pub struct IndexedMesh {
    pub vbo: Subbuffer<[PosColVertex]>,
    pub ibo: Subbuffer<[u32]>,
}

enum GlobalEvent {
    Update,
}

fn main() {
    //std::env::set_var("RUST_BACKTRACE", "1");

    let event_loop = EventLoopBuilder::<GlobalEvent>::with_user_event()
        .build()
        .unwrap();

    let (mut renderer, _main_window_id) = Renderer::new(&event_loop);

    let pass_ubo = Arc::new(Mutex::new(SubbufferAllocator::new(
        renderer.allocator().clone(),
        SubbufferAllocatorCreateInfo {
            buffer_usage: BufferUsage::UNIFORM_BUFFER,
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
    )));

    let obj_ubo = Arc::new(Mutex::new(SubbufferAllocator::new(
        renderer.allocator().clone(),
        SubbufferAllocatorCreateInfo {
            buffer_usage: BufferUsage::UNIFORM_BUFFER,
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
    )));

    //let (surface_format, num_frames_in_flight) = {
    //    let guard = renderer.windows.get(&main_window_id).unwrap().lock().unwrap();
    //    (guard.surface_image_format, guard.num_frames_in_flight)
    //};

    let renderpass = vulkano::single_pass_renderpass!(
        renderer.device().clone(),
        attachments: {
            color: {
                format: Format::R8G8B8A8_SRGB,
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            depth: {
                format: Format::D32_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth},
        },
    )
    .unwrap();

    let canvas = Canvas::empty(
        renderpass.clone(),
        [
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_SRC,
                ..Default::default()
            },
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
        ]
        .into(),
    );

    let pipeline = {
        let vs = {
            let mut bytes = Vec::new();
            let mut file = std::fs::File::open("src/shaders/triangle/triangle.vert.spv").unwrap();
            file.read_to_end(&mut bytes).unwrap();
            let spirv: Vec<u32> = vulkano::shader::spirv::bytes_to_words(&bytes)
                .unwrap()
                .into_owned();
            let module = unsafe {
                ShaderModule::new(
                    renderer.device().clone(),
                    ShaderModuleCreateInfo::new(&spirv),
                )
            }
            .unwrap();
            module.entry_point("main").unwrap()
        };

        let fs = {
            let mut bytes = Vec::new();
            let mut file = std::fs::File::open("src/shaders/triangle/triangle.frag.spv").unwrap();
            file.read_to_end(&mut bytes).unwrap();
            let spirv: Vec<u32> = vulkano::shader::spirv::bytes_to_words(&bytes)
                .unwrap()
                .into_owned();
            let module = unsafe {
                ShaderModule::new(
                    renderer.device().clone(),
                    ShaderModuleCreateInfo::new(&spirv),
                )
            }
            .unwrap();
            module.entry_point("main").unwrap()
        };

        //let vertex_input_state = VertexInputState::new()
        //    .binding(0, VertexInputBindingDescription {
        //        stride: std::mem::size_of::<PosColVertex>() as u32,
        //        input_rate: VertexInputRate::Vertex
        //    })
        //    .attribute(0, VertexInputAttributeDescription {
        //        binding: 0,
        //        format: Format::R32G32_SFLOAT,
        //        offset: std::mem::offset_of!(PosColVertex, position) as u32
        //    })
        //    .attribute(1, VertexInputAttributeDescription {
        //        binding: 0,
        //        format: Format::R32G32B32_SFLOAT,
        //        offset: std::mem::offset_of!(PosColVertex, color) as u32
        //    });

        let vertex_input_state = {
            let info = PosColVertex::per_vertex();
            let input_state = VertexInputState::new().binding(
                0,
                VertexInputBindingDescription {
                    stride: info.stride,
                    input_rate: info.input_rate,
                },
            );

            let mut members = info.members.iter().collect::<Vec<_>>();
            members.sort_by_key(|(_, member)| member.offset);

            let members = members.iter().enumerate().map(|(i, (_, member))| {
                //println!("member \"{}\" ({}):\n{:#?}", name, i, member);
                (
                    i as u32,
                    VertexInputAttributeDescription {
                        binding: 0,
                        format: member.format,
                        offset: member.offset as u32,
                    },
                )
            });

            input_state.attributes(members)
        };

        //let vertex_input_state = Vertex::per_vertex().definition(&vs.info().input_interface).unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        //let layout = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages);
        //println!("layout: \n{:#?}", layout);

        let set_layouts = vec![
            {
                // Per frame
                DescriptorSetLayoutCreateInfo {
                    flags: DescriptorSetLayoutCreateFlags::empty(),
                    bindings: [].into(),
                    ..Default::default()
                }
            },
            {
                // Per pass
                DescriptorSetLayoutCreateInfo {
                    flags: DescriptorSetLayoutCreateFlags::empty(),
                    bindings: [(0, {
                        let mut binding = DescriptorSetLayoutBinding::descriptor_type(
                            DescriptorType::UniformBuffer,
                        );
                        binding.stages = ShaderStages::VERTEX;
                        binding
                    })]
                    .into(),
                    ..Default::default()
                }
            },
            {
                // Material
                DescriptorSetLayoutCreateInfo {
                    flags: DescriptorSetLayoutCreateFlags::empty(),
                    bindings: [].into(),
                    ..Default::default()
                }
            },
            {
                // Objects
                DescriptorSetLayoutCreateInfo {
                    flags: DescriptorSetLayoutCreateFlags::empty(),
                    bindings: [(0, {
                        let mut binding = DescriptorSetLayoutBinding::descriptor_type(
                            DescriptorType::UniformBuffer,
                        );
                        binding.stages = ShaderStages::VERTEX;
                        binding
                    })]
                    .into(),
                    ..Default::default()
                }
            },
        ];

        let layout = PipelineLayout::new(
            renderer.device().clone(),
            PipelineDescriptorSetLayoutCreateInfo {
                flags: PipelineLayoutCreateFlags::empty(),
                set_layouts,
                push_constant_ranges: Vec::new(),
            }
            .into_pipeline_layout_create_info(renderer.device().clone())
            .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(renderpass.clone(), 0).unwrap();

        GraphicsPipeline::new(
            renderer.device().clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState {
                    cull_mode: CullMode::None,
                    front_face: FrontFace::CounterClockwise,
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    };

    let hex_mesh = {
        let mut verts: Vec<PosColVertex> = Vec::new();

        verts.push(PosColVertex {
            position: [0.0, 0.0],
            color: [0.0, 0.0, 0.0],
        });
        let num_points = 6;
        let radius = 0.5;
        for i in 0..num_points {
            let i = i as f32;
            let x = radius * ((6.283 / num_points as f32) * i - (6.283 / 4.0)).cos();
            let y = radius * ((6.283 / num_points as f32) * i - (6.283 / 4.0)).sin();

            verts.push(PosColVertex {
                position: [x, y],
                color: [0.0, 0.0, 0.0],
            })
        }

        let mut inds: Vec<u32> = Vec::new();
        for i in 1..num_points {
            inds.push(i);
            inds.push(0);
            inds.push(i + 1);
        }

        inds.push(num_points);
        inds.push(0);
        inds.push(1);

        let vbo = Buffer::from_iter(
            renderer.allocator().clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            verts,
        )
        .unwrap();

        let ibo = Buffer::from_iter(
            renderer.allocator().clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            inds.clone(),
        )
        .unwrap();

        IndexedMesh { vbo, ibo }
    };

    let meshes: HashMap<&'static str, IndexedMesh> = [("hex", hex_mesh)].into();

    let start_time = Instant::now();

    let proxy = event_loop.create_proxy();
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent { window_id, event } => match event {
                    WindowEvent::CloseRequested => {
                        _ = renderer.windows.remove(&window_id);
                        if renderer.windows.len() == 0 {
                            elwt.exit()
                        }
                    }
                    WindowEvent::Resized(_) => {
                        let windows = &mut renderer.windows;
                        let mut window = windows.get_mut(&window_id).unwrap().lock().unwrap();
                        window.recreate_swapchain = true;
                    }
                    WindowEvent::RedrawRequested => {
                        let rendersystem = DefaultRenderSystem::new(
                            PresentSystem {
                                window: renderer.windows.get(&window_id).unwrap().clone(),
                            }
                            .into(),
                            vec![
                                CirclesRenderPass {
                                    elapsed_time: Instant::now()
                                        .duration_since(start_time)
                                        .as_secs_f32(),
                                    pass_ubo: pass_ubo.clone(),
                                    obj_ubo: obj_ubo.clone(),
                                    pipeline: pipeline.clone(),
                                    meshes: meshes.clone(),
                                    canvas: canvas.clone(),
                                }
                                .into(),
                                WindowBlitRenderPass {
                                    src_canvas: canvas.clone(),
                                    attachment_index: 0,
                                }
                                .into(),
                            ],
                        );

                        let mut barrier = renderer.comms.send(rendersystem);

                        _ = proxy.send_event(GlobalEvent::Update);

                        barrier.blocking_wait();
                    }
                    _ => (),
                },
                Event::AboutToWait => {
                    let windows = &renderer.windows;
                    let barriers: Vec<_> = windows
                        .iter()
                        .map(|(_, w)| {
                            let rendersystem = DefaultRenderSystem::new(
                                PresentSystem { window: w.clone() }.into(),
                                vec![
                                    CirclesRenderPass {
                                        elapsed_time: Instant::now()
                                            .duration_since(start_time)
                                            .as_secs_f32(),
                                        pass_ubo: pass_ubo.clone(),
                                        obj_ubo: obj_ubo.clone(),
                                        pipeline: pipeline.clone(),
                                        meshes: meshes.clone(),
                                        canvas: canvas.clone(),
                                    }
                                    .into(),
                                    WindowBlitRenderPass {
                                        src_canvas: canvas.clone(),
                                        attachment_index: 0,
                                    }
                                    .into(),
                                ],
                            );

                            renderer.comms.send(rendersystem)
                        })
                        .collect();

                    _ = proxy.send_event(GlobalEvent::Update);

                    barriers.into_iter().for_each(|mut b| b.blocking_wait())
                }
                Event::UserEvent(event) => match event {
                    GlobalEvent::Update => {}
                },
                _ => (),
            }
        })
        .unwrap();
}
