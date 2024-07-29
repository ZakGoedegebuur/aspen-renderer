use crate::{renderpass::{CmdBuffer, HaltPolicy}, GraphicsObjects};

pub trait SubmitSystem {
    type SharedType;
    type SetupType;
    fn setup(&mut self, graphics_objects: &GraphicsObjects) -> Result<(Self::SharedType, Self::SetupType, Box<CmdBuffer>), HaltPolicy>;
    fn submit(&mut self, graphics_objects: &GraphicsObjects, cmd_buffer: Box<CmdBuffer>, setup_data: Self::SetupType);
}

trait SubmitSystemCont {
    type SharedType;
    fn setup(&mut self, graphics_objects: &GraphicsObjects) -> Result<(Self::SharedType, Box<CmdBuffer>), HaltPolicy>;
    fn submit(&mut self, graphics_objects: &GraphicsObjects, cmd_buffer: Box<CmdBuffer>);
}

pub struct DynamicSubmitSystem<T: SubmitSystem> {
    inner: T,
    data: Option<T::SetupType>,
}

impl<T: SubmitSystem> SubmitSystemCont for DynamicSubmitSystem<T> {
    type SharedType = T::SharedType;

    fn setup(&mut self, graphics_objects: &GraphicsObjects) -> Result<(T::SharedType, Box<CmdBuffer>), HaltPolicy> {
        let (shared, data, cmd_buf) = self.inner.setup(graphics_objects)?;
        self.data = Some(data);
        Ok((shared, cmd_buf))
    }

    fn submit(&mut self, graphics_objects: &GraphicsObjects, cmd_buffer: Box<CmdBuffer>) {
        self.inner.submit(graphics_objects, cmd_buffer, self.data.take().unwrap())
    }
}