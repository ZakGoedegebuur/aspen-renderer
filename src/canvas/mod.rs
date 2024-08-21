use std::sync::{
    Arc,
    Mutex,
};

use vulkano::{
    command_buffer::RenderPassBeginInfo,
    format::ClearValue,
    image::{
        view::ImageView,
        Image,
        ImageCreateInfo,
    },
    memory::allocator::{
        AllocationCreateInfo,
        MemoryAllocator,
    },
    render_pass::{
        Framebuffer,
        FramebufferCreateInfo,
        RenderPass,
    },
    ValidationError,
};

use crate::renderpass::CmdBuffer;

pub struct Canvas {
    pub inner: Mutex<CanvasInner>,
}

#[derive(Debug)]
pub struct CanvasInner {
    renderpass: Arc<RenderPass>,
    image_create_infos: Vec<ImageCreateInfo>,
    num_frames_in_flight: usize,
    current_set: usize,
    image_sets: Vec<Vec<Arc<ImageView>>>,
    framebuffers: Vec<Arc<Framebuffer>>,
}

impl Canvas {
    pub fn empty(
        renderpass: Arc<RenderPass>,
        image_create_infos: Vec<ImageCreateInfo>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(CanvasInner {
                renderpass,
                image_create_infos,
                num_frames_in_flight: 0,
                current_set: 0,
                image_sets: Vec::new(),
                framebuffers: Vec::new(),
            }),
        })
    }

    pub fn extent(self: &Arc<Self>) -> [u32; 2] {
        let guard = self.inner.lock().unwrap();
        match guard.framebuffers.get(0) {
            None => [0, 0],
            Some(fb) => fb.extent(),
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
    pub fn recreate_buffers_exact(
        self: &Arc<Self>,
        exact_extent: [u32; 3],
        num_frames_in_flight: usize,
        allocator: Arc<dyn MemoryAllocator>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        inner.recreate_buffers_exact(exact_extent, num_frames_in_flight, allocator);
    }

    pub fn pass_controller(self: &Arc<Self>) -> RenderPassController {
        let mut inner = self.inner.lock().unwrap();
        inner.current_set += 1;
        inner.current_set %= inner.num_frames_in_flight;

        RenderPassController {
            current_subpass: None,
            image_views: inner.image_sets[inner.current_set].clone(),
            framebuffer: inner.framebuffers[inner.current_set].clone(),
        }
    }
}

pub struct RenderPassController {
    current_subpass: Option<usize>,
    pub framebuffer: Arc<Framebuffer>,
    pub image_views: Vec<Arc<ImageView>>,
}

impl RenderPassController {
    pub fn begin_renderpass<'a>(
        &'a mut self,
        cmd_buf: &'a mut CmdBuffer,
        clear_values: Vec<Option<ClearValue>>,
    ) -> Result<&mut CmdBuffer, Box<ValidationError>> {
        match cmd_buf.begin_render_pass(
            RenderPassBeginInfo {
                clear_values,
                ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
            },
            Default::default(),
        ) {
            Ok(val) => {
                self.current_subpass = Some(0);
                Ok(val)
            }
            Err(err) => Err(err),
        }
    }

    pub fn begin_renderpass_with_extent<'a>(
        &'a mut self,
        cmd_buf: &'a mut CmdBuffer,
        clear_values: Vec<Option<ClearValue>>,
        extent: [u32; 2],
        offset: [u32; 2],
    ) -> Result<&mut CmdBuffer, Box<ValidationError>> {
        match cmd_buf.begin_render_pass(
            RenderPassBeginInfo {
                clear_values,
                render_area_extent: extent,
                render_area_offset: offset,
                ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
            },
            Default::default(),
        ) {
            Ok(val) => {
                self.current_subpass = Some(0);
                Ok(val)
            }
            Err(err) => Err(err),
        }
    }

    pub fn next_subpass<'a>(
        &'a mut self,
        cmd_buf: &'a mut CmdBuffer,
    ) -> Result<&mut CmdBuffer, Box<ValidationError>> {
        *self
            .current_subpass
            .as_mut()
            .expect("renderpass not active") += 1;
        cmd_buf.next_subpass(Default::default(), Default::default())
    }

    pub fn end_renderpass(
        self,
        cmd_buf: &mut CmdBuffer,
    ) -> Result<&mut CmdBuffer, Box<ValidationError>> {
        cmd_buf.end_render_pass(Default::default())
    }
}

impl CanvasInner {
    pub fn recreate_buffers_exact(
        &mut self,
        exact_extent: [u32; 3],
        num_frames_in_flight: usize,
        allocator: Arc<dyn MemoryAllocator>,
    ) {
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
                            AllocationCreateInfo::default(),
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                )
            }

            self.framebuffers.push(
                Framebuffer::new(
                    self.renderpass.clone(),
                    FramebufferCreateInfo {
                        attachments: set.clone(),
                        ..Default::default()
                    },
                )
                .unwrap(),
            );

            self.image_sets.push(set);
        }

        //println!("recreate_buffers_exact:\n{:#?}", self)
    }
}
