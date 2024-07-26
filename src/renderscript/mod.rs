use std::sync::{Arc, Mutex};

use crate::{window_surface::WindowSurface, GraphicsObjects};

pub struct RenderScript {
    script: Box<dyn FnOnce(&GraphicsObjects) + Send>,
}

impl RenderScript {
    pub fn new(script: impl FnOnce(&GraphicsObjects) + Send + 'static) -> Self {
        Self {
            script: Box::new(script),
        }
    }

    pub fn run(self, graphics_objects: &GraphicsObjects) {
        (self.script)(graphics_objects)
    }
}