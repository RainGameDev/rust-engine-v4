use egui::Context;
use macros::Resource;

#[derive(Resource, Debug, Clone)]
pub struct EguiContext(pub Context);
