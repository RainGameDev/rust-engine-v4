pub mod assets;
pub mod ecs;
pub mod ffi;
pub mod input;
pub mod logging;
pub mod rendering;
pub mod time;
pub mod window;

use anyhow::Result;

/// Engine handler
pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init_core() -> Result<()> {
    let engine = Engine::new();
    window::run(engine)
}
