use std::path::Path;

use anyhow::Result;
use macros::fixed_update;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

use crate::{
    Engine,
    ecs::{
        components::engine_components::transform::transform_update,
        systems::{StartSystem, run_system, scheduler::Schedule},
    },
    log_error,
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
                self.schedule.tick(&mut self.engine.ecs_world).unwrap();
                if let Some(renderer) = &mut self.renderer {
                    transform_update(&mut self.engine.ecs_world);

                    let context = &self.rendering_info.as_ref().unwrap().context;
                    let gpu_mesh = load_gltf_file(
                        &Path::new(&format!(
                            "{}/{}meshes/test.glb",
                            env!("CARGO_MANIFEST_DIR"),
                            "res/"
                        )),
                        context,
                        renderer.command_pool,
                    )
                    .unwrap();

                    let Some(mut frame_info) = self.engine.return_renderable() else {
                        log_error!("No active camera, not rendering");
                        return;
                    };

                    for mesh in gpu_mesh {
                        frame_info.meshes.push(mesh);
                    }

                    renderer.render(frame_info).unwrap();
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
