pub mod assets;
pub mod ecs;
pub mod ffi;
pub mod input;
pub mod logging;
pub mod rendering;
pub mod time;
pub mod window;

#[cfg(test)]
mod tests;

pub use inventory;
pub use macros::Component;

use anyhow::Result;

/// Engine handler
pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn return_renderable() {}
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
