use std::sync::Arc;

use crate::{renderpass::{CmdBuffer, HaltPolicy}, GraphicsObjects};

pub trait SubmitSystem {
    type SharedType;
    type SetupType;
    fn setup(&mut self, graphics_objects: Arc<GraphicsObjects>) -> Result<(Arc<Self::SharedType>, Self::SetupType, Box<CmdBuffer>), HaltPolicy>;
    fn submit(&mut self, graphics_objects: Arc<GraphicsObjects>, cmd_buffer: Box<CmdBuffer>, setup_data: Self::SetupType, shared_data: Arc<Self::SharedType>);
}

pub trait SubmitSystemCont {
    type SharedType;
    fn setup(&mut self, graphics_objects: Arc<GraphicsObjects>) -> Result<(Arc<Self::SharedType>, Box<CmdBuffer>), HaltPolicy>;
    fn submit(&mut self, graphics_objects: Arc<GraphicsObjects>, cmd_buffer: Box<CmdBuffer>, shared_data: Arc<Self::SharedType>);
}

pub struct DynamicSubmitSystem<T: SubmitSystem> {
    inner: T,
    data: Option<T::SetupType>,
}

impl<T: SubmitSystem> DynamicSubmitSystem<T> {
    pub fn new(system: T) -> Self {
        DynamicSubmitSystem {
            inner: system,
            data: None,
        }
    }
}

impl<T: SubmitSystem> From<T> for DynamicSubmitSystem<T> {
    fn from(value: T) -> Self {
        DynamicSubmitSystem { 
            inner: value, 
            data: None, 
        }
    }
}

impl<T: SubmitSystem> SubmitSystemCont for DynamicSubmitSystem<T> {
    type SharedType = T::SharedType;

    fn setup(&mut self, graphics_objects: Arc<GraphicsObjects>) -> Result<(Arc<T::SharedType>, Box<CmdBuffer>), HaltPolicy> {
        let (shared, setup, cmd_buf) = self.inner.setup(graphics_objects)?;
        self.data = Some(setup);
        Ok((shared, cmd_buf))
    }

    fn submit(&mut self, graphics_objects: Arc<GraphicsObjects>, cmd_buffer: Box<CmdBuffer>, shared_data: Arc<T::SharedType>) {
        self.inner.submit(graphics_objects, cmd_buffer, self.data.take().unwrap(), shared_data)
    }
}

