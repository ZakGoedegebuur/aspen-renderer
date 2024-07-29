use crate::renderpass::CmdBuffer;

pub trait Drawable {
    fn draw(&mut self, command_buffer: &mut CmdBuffer);
}