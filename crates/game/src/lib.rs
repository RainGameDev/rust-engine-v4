use anyhow::Result;
use engine_core::update;

#[update]
pub fn update() -> Result<()> {
    Ok(())
}
pub fn init() {}
