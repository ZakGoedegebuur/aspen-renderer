use std::sync::{Arc, Mutex};

use vulkano::{command_buffer::RenderPassBeginInfo, image::{view::ImageView, Image, ImageCreateInfo}, memory::allocator::{AllocationCreateInfo, MemoryAllocator}, render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass}, ValidationError, VulkanError};

use crate::renderpass::CmdBuffer;

pub struct Canvas {
    inner: Mutex<CanvasInner>
}

#[derive(Debug)]
struct CanvasInner {
    renderpass: Arc<RenderPass>,
    image_create_infos: Vec<ImageCreateInfo>,
    num_frames_in_flight: usize,
    current_set: usize, 
    image_sets: Vec<Vec<Arc<ImageView>>>,
    framebuffers: Vec<Arc<Framebuffer>>,
}

impl Canvas {
    pub fn empty(renderpass: Arc<RenderPass>, image_create_infos: Vec<ImageCreateInfo>) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(CanvasInner {
                renderpass,
                image_create_infos,
                num_frames_in_flight: 0,
                current_set: 0,
                image_sets: Vec::new(),
                framebuffers: Vec::new(),
            })
        })
    }

    pub fn extent(self: &Arc<Self>) -> [u32; 2] {
        let guard = self.inner.lock().unwrap();
        match guard.framebuffers.get(0) {
            None => [0, 0],
            Some(fb) => fb.extent()
        }
    }

    pub fn current_image_set(self: &Arc<Self>) -> Vec<Arc<ImageView>> {
        let inner = self.inner.lock().unwrap();
        inner.image_sets[inner.current_set].clone()
    }

    /* TODO
    /// Makes sure images can fit the min extent, and if not, recreates them
    pub fn recreate_buffers(&mut self, min_extent: [u32; 3]) {
    }
    */

    /// Recreate buffers, making sure the images fit the extent precisely
    pub fn recreate_buffers_exact(self: &Arc<Self>, exact_extent: [u32; 3], num_frames_in_flight: usize, allocator: Arc<dyn MemoryAllocator>) {
        let mut inner = self.inner.lock().unwrap();
        inner.recreate_buffers_exact(exact_extent, num_frames_in_flight, allocator);
    }

    pub fn begin_renderpass(self: &Arc<Self>, cmd_buf: &mut CmdBuffer) -> Result<RenderPassController, Box<vulkano::ValidationError>> {
        let mut inner = self.inner.lock().unwrap();
        inner.current_set += 1;
        inner.current_set %= inner.num_frames_in_flight;
        match cmd_buf.begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![Some([0.07, 0.07, 0.07, 1.0].into()), Some(1.0.into())],
                ..RenderPassBeginInfo::framebuffer(
                    inner.framebuffers[inner.current_set].clone()
                )
            },
            Default::default(),
        ) {
            Ok(_) => {
                Ok(
                    RenderPassController {
                        current_subpass: 0
                    }
                )
            },
            Err(err) => Err(err)
        }
    }
}

pub struct RenderPassController {
    current_subpass: usize,
}

impl RenderPassController {
    pub fn next_subpass(&mut self, cmd_buf: &mut CmdBuffer) -> Result<(), Box<ValidationError>> {
        self.current_subpass += 1;
        match cmd_buf.next_subpass(Default::default(), Default::default()) {
            Ok(_) => Ok(()),
            Err(err) => Err(err)
        }
    }

    pub fn end(self, cmd_buf: &mut CmdBuffer) -> Result<(), Box<ValidationError>> {
        match cmd_buf.end_render_pass(Default::default()) {
            Ok(_) => Ok(()),
            Err(err) => Err(err)
        }
    }
}

impl CanvasInner {
    pub fn recreate_buffers_exact(&mut self, exact_extent: [u32; 3], num_frames_in_flight: usize, allocator: Arc<dyn MemoryAllocator>) {
        self.num_frames_in_flight = num_frames_in_flight;
        self.image_sets.clear();
        self.framebuffers.clear();

        for _ in 0..self.num_frames_in_flight {
            let mut set = Vec::new();

            for create_info in self.image_create_infos.iter().cloned() {
                set.push(
                    ImageView::new_default(
                        Image::new(
                            allocator.clone(), 
                            ImageCreateInfo {
                                extent: exact_extent,
                                ..create_info
                            }, 
                            AllocationCreateInfo::default()
                        ).unwrap()
                    ).unwrap()
                )
            }

            self.framebuffers.push(
                Framebuffer::new(
                    self.renderpass.clone(), 
                    FramebufferCreateInfo {
                        attachments: set.clone(),
                        ..Default::default()
                    }
                ).unwrap()
            );
            
            self.image_sets.push(set);
        }

        //println!("recreate_buffers_exact:\n{:#?}", self)
    }
}