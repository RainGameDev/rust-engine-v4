use std::path::Path;

use anyhow::Result;
use nalgebra::Vector3;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

use crate::{
    Engine,
    ecs::{
        components::engine_components::{model_renderer::ModelRenderer, transform::Transform},
        query::query::Query,
        systems::{StartSystem, run_system, scheduler::Schedule},
    },
    log_error, log_warn,
    rendering::{core::model::GpuMesh, egui::context::EguiContext},
    utils::directory_check::load_directory,
};
use crate::{
    assets::models::gltf::load_gltf_file,
    rendering::{
        core::frame_info::update_camera_aspect_ratio,
        vulkan::{RenderingInfo, VulkanRenderer},
    },
};

/// App that runs the window, and engine
pub struct App {
    rendering_info: Option<RenderingInfo>,
    renderer: Option<VulkanRenderer>,
    engine: Engine,
    schedule: Schedule,
}

impl App {
    pub fn new(engine: Engine) -> Self {
        Self {
            rendering_info: None,
            renderer: None,
            engine,
            // set to the refresh rate for physics and such
            schedule: Schedule::new(60.0),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.rendering_info = Some(RenderingInfo::new(event_loop));
        self.renderer = Some(VulkanRenderer::new(self.rendering_info.clone().unwrap()).unwrap());
        let context = &self.rendering_info.as_ref().unwrap().context;
        let command_pool = self.renderer.as_ref().unwrap().command_pool;

        let model_paths = load_directory(
            Path::new(&format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "res/")),
            "glb",
        )
        .unwrap();

        for model_path in model_paths {
            match load_gltf_file(Path::new(&model_path), context, command_pool) {
                Ok(mesh) => {
                    for mesh in mesh {
                        let handle = self.engine.ecs_world.add_asset(mesh, model_path.clone());
                        crate::log_info!("loaded mesh: {} -> {:?}", model_path, handle);
                    }
                }
                Err(err) => {
                    crate::log_error!(reason: "failed to load gltf", "{}: {err:?}", model_path);
                }
            }
        }

        let context = EguiContext(self.renderer.as_ref().unwrap().get_egui_context());
        self.engine.ecs_world.add_resource(context);

        let model = self.engine.ecs_world.spawn();
        self.engine
            .ecs_world
            .add_component(model, Transform::from_position(Vector3::new(0.0, 0.0, 0.0)));

        let model_asset = self
            .engine
            .ecs_world
            .get_asset_handle::<GpuMesh>("meshes/test.glb")
            .unwrap();
        self.engine
            .ecs_world
            .add_component(model, ModelRenderer { model: model_asset });
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.rendering_info.is_none() {
            return;
        }

        if let Some(renderer) = &mut self.renderer {
            let _ = renderer.handle_ui_event(&event.clone());
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize().unwrap();
                    update_camera_aspect_ratio(
                        &mut self.engine.ecs_world,
                        renderer.swapchain.extent.width,
                        renderer.swapchain.extent.height,
                    );
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.begin_ui();
                    self.schedule.tick(&mut self.engine.ecs_world).unwrap();

                    let Some(mut frame_info) = self.engine.return_renderable() else {
                        log_error!("No active camera, not rendering");
                        return;
                    };

                    let query: Query<(&ModelRenderer, &Transform)> =
                        Query::new(&self.engine.ecs_world);

                    for (model_renderer, _) in query.iter() {
                        let Some(mesh) = self.engine.ecs_world.get_asset(model_renderer.model)
                        else {
                            log_warn!(reason: "stale or missing mesh handle", "skipping entity");
                            continue;
                        };

                        frame_info.meshes.push(mesh.clone());
                    }

                    renderer.render(frame_info).unwrap();
                    renderer.end_ui().unwrap();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(info) = &self.rendering_info {
            info.window.request_redraw();
        }
    }
}

pub fn run(engine: Engine) -> Result<()> {
    let mut app = App::new(engine);
    run_system(&mut app.engine.ecs_world, StartSystem::sorted())?;
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
