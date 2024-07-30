use std::{collections::{BTreeMap, HashMap}, io::Read, sync::{Arc, Mutex}, time::Instant};

use aspen_renderer::{render_system::DefaultRenderSystem, Renderer};
use passes::{circles::CirclesRenderPass, present::PresentSystem};
use vulkano::{
    buffer::{allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo}, Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, descriptor_set::layout::{DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags, DescriptorSetLayoutCreateInfo, DescriptorType}, format::Format, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}, pipeline::{graphics::{color_blend::{ColorBlendAttachmentState, ColorBlendState}, input_assembly::InputAssemblyState, multisample::MultisampleState, rasterization::{CullMode, FrontFace, RasterizationState}, vertex_input::{Vertex, VertexInputAttributeDescription, VertexInputBindingDescription, VertexInputRate, VertexInputState}, viewport::ViewportState, GraphicsPipelineCreateInfo}, layout::{PipelineDescriptorSetLayoutCreateInfo, PipelineLayoutCreateFlags}, DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo}, render_pass::Subpass, shader::{ShaderModule, ShaderModuleCreateInfo, ShaderStages}};
use winit::{
    event::{
        Event, 
        WindowEvent
    }, 
    event_loop::{
        ControlFlow, 
        EventLoopBuilder
    }
};

mod passes;

pub struct RenderData {
    pub elapsed_time: f32,
    pub ubo: Arc<Mutex<SubbufferAllocator>>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub meshes: HashMap<&'static str, IndexedMesh>
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
    pub ibo: Subbuffer<[u32]>
}

enum GlobalEvent {
    Update,
}


fn main() {
    //std::env::set_var("RUST_BACKTRACE", "1");``
    
    let event_loop = EventLoopBuilder::<GlobalEvent>::with_user_event().build().unwrap();

    let (mut renderer, main_window_id) = Renderer::new(&event_loop);

    let uniform_buffer = Arc::new(Mutex::new(SubbufferAllocator::new(
        renderer.allocator().clone(), 
        SubbufferAllocatorCreateInfo {
            buffer_usage: BufferUsage::UNIFORM_BUFFER,
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        }
    )));

    let pipeline = {
        let vs = {
            let mut bytes = Vec::new();
            let mut file = std::fs::File::open("src/shaders/triangle/triangle.vert.spv").unwrap();
            file.read_to_end(&mut bytes).unwrap();
            let spirv: Vec<u32> = vulkano::shader::spirv::bytes_to_words(&bytes).unwrap().into_owned();
            let module = unsafe {
                ShaderModule::new(renderer.device().clone(), ShaderModuleCreateInfo::new(&spirv))
            }.unwrap();
            module.entry_point("main").unwrap()
        };

        let fs = {
            let mut bytes = Vec::new();
            let mut file = std::fs::File::open("src/shaders/triangle/triangle.frag.spv").unwrap();
            file.read_to_end(&mut bytes).unwrap();
            let spirv: Vec<u32> = vulkano::shader::spirv::bytes_to_words(&bytes).unwrap().into_owned();
            let module = unsafe {
                ShaderModule::new(renderer.device().clone(), ShaderModuleCreateInfo::new(&spirv))
            }.unwrap();
            module.entry_point("main").unwrap()
        };

        let vertex_input_state = VertexInputState::new()
            .binding(0, VertexInputBindingDescription {
                stride: std::mem::size_of::<PosColVertex>() as u32,
                input_rate: VertexInputRate::Vertex
            })
            .attribute(0, VertexInputAttributeDescription {
                binding: 0,
                format: Format::R32G32_SFLOAT,
                offset: std::mem::offset_of!(PosColVertex, position) as u32
            })
            .attribute(1, VertexInputAttributeDescription {
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: std::mem::offset_of!(PosColVertex, color) as u32
            });

        //let vertex_input_state = Vertex::per_vertex().definition(&vs.info().input_interface).unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        //let layout = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages);
        //println!("layout: \n{:#?}", layout);

        let mut set_layouts = Vec::new();
        set_layouts.push(
            {
                let mut set = DescriptorSetLayoutCreateInfo {
                    flags: DescriptorSetLayoutCreateFlags::empty(),
                    bindings: BTreeMap::new(),
                    ..Default::default()
                };

                let mut binding = DescriptorSetLayoutBinding::descriptor_type(DescriptorType::UniformBuffer);
                binding.stages = ShaderStages::VERTEX;

                set.bindings.insert(0, binding);
                set
            }
        );

        let layout = PipelineLayout::new(
            renderer.device().clone(),
            PipelineDescriptorSetLayoutCreateInfo {
                flags: PipelineLayoutCreateFlags::empty(),
                set_layouts,
                push_constant_ranges: Vec::new(),
            }.into_pipeline_layout_create_info(renderer.device().clone()).unwrap(),
        )
        .unwrap();

        let renderpass = {
            let guard = renderer.windows.get(&main_window_id).unwrap().lock().unwrap();
            guard.renderpass.clone()
        };

        let subpass = Subpass::from(renderpass, 0).unwrap();

        GraphicsPipeline::new(
            renderer.device().clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState {
                    cull_mode: CullMode::Back,
                    front_face: FrontFace::CounterClockwise,
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    };

    

    let hex_mesh = {
        let mut verts: Vec<PosColVertex> = Vec::new();

        verts.push(PosColVertex { position: [0.0, 0.0], color: [0.0, 0.0, 0.0] });
        let num_points = 6;
        let radius = 0.5;
        for i in 0..num_points {
            let i = i as f32;
            let x = radius * ((6.283 / num_points as f32) * i - (6.283 / 4.0)).cos();
            let y = radius * ((6.283 / num_points as f32) * i - (6.283 / 4.0)).sin();

            verts.push(PosColVertex { position: [x, y], color: [0.0, 0.0, 0.0] })
        }

        let mut inds: Vec<u32> = Vec::new();
        for i in 1..num_points  {
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
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            }, 
            inds.clone()
        )
        .unwrap();

        IndexedMesh {
            vbo,
            ibo
        }
    };

    let meshes: HashMap<&'static str, IndexedMesh> = [("hex", hex_mesh)].into();

    let start_time = Instant::now();

    let proxy = event_loop.create_proxy();
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent {
                window_id, 
                event 
            } => match event {
                WindowEvent::CloseRequested => {
                    _ = renderer.windows.remove(&window_id);
                    if renderer.windows.len() == 0 {
                        elwt.exit()
                    }
                },
                WindowEvent::Resized(_) => {
                    let windows = &mut renderer.windows;
                    let mut window = windows.get_mut(&window_id).unwrap().lock().unwrap();
                    window.recreate_swapchain = true;
                },
                WindowEvent::RedrawRequested => {
                    let rendersystem = DefaultRenderSystem::new(
                        PresentSystem {
                            window: renderer.windows.get(&window_id).unwrap().clone()
                        }.into(),
                        vec![
                            CirclesRenderPass {
                                elapsed_time: Instant::now().duration_since(start_time).as_secs_f32(),
                                ubo: uniform_buffer.clone(),
                                pipeline: pipeline.clone(),
                                meshes: meshes.clone()
                            }.into()
                        ]
                    );

                    let mut barrier = renderer.comms.send(rendersystem);

                    _ = proxy.send_event(GlobalEvent::Update);
                    
                    barrier.blocking_wait();
                },
                _ => ()
            },
            Event::AboutToWait => {
                let windows = &renderer.windows;
                let barriers: Vec<_> = windows
                    .iter()
                    .map(|(_, w)| {
                        let rendersystem = DefaultRenderSystem::new(
                            PresentSystem {
                                window: w.clone()
                            }.into(),
                            vec![
                                CirclesRenderPass {
                                    elapsed_time: Instant::now().duration_since(start_time).as_secs_f32(),
                                    ubo: uniform_buffer.clone(),
                                    pipeline: pipeline.clone(),
                                    meshes: meshes.clone()
                                }.into()
                            ]
                        );

                        renderer.comms.send(rendersystem)
                    })
                    .collect();

                _ = proxy.send_event(GlobalEvent::Update);
                
                barriers.into_iter().for_each(|mut b| { b.blocking_wait() })
            }
            Event::UserEvent(event) => match event {
                GlobalEvent::Update => {
                },
            }
            _ => ()
        }
    }).unwrap();
}