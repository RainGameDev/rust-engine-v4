use engine_core::init_core;
pub mod registry;

fn main() -> anyhow::Result<()> {
    game::init();
    init_core(Some("127.0.0.1:5000".parse()?))
}
