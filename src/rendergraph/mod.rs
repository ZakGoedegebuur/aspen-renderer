pub struct RenderGraph {
    root_renderpass: Option<Box<dyn RenderNode + Send + Sync>>
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            root_renderpass: None
        }
    }

    pub fn render(&self) {

    }

    fn depthwise_recurse() {

    }
}

pub trait RenderNode {
    fn execute(&self);
}

mod render_passes {
    
}