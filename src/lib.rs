mod rendergraph;

use std::{
    sync::mpsc, 
    thread
};

pub use rendergraph::RenderGraph;

pub struct Renderer {
    render_thread: thread::JoinHandle<()>,
    sender: mpsc::Sender<RenderGraph>,
}

impl Renderer {
    pub fn new() -> Self {
        let (sender, reciever) = mpsc::channel::<RenderGraph>();
        let render_thread = thread::spawn(move || {
            let reciever = reciever;

            loop {
                match reciever.recv() {
                    Err(_) => break,
                    Ok(rendergraph) => {
                        
                    },
                }
            }

            
        });
        
        Self {
            render_thread,
            sender,
        }
    }
}
