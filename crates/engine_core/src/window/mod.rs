use anyhow::Result;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

use crate::rendering::{
    core::{frame_info::update_camera_aspect_ratio, model::raw_mesh_to_gpu_mesh},
    vulkan::{RenderingInfo, VulkanRenderer},
};
use crate::{Engine, rendering::core::model::cube_mesh};

/// App that runs the window, and engine
pub struct App {
    rendering_info: Option<RenderingInfo>,
    renderer: Option<VulkanRenderer>,
    engine: Engine,
}

impl App {
    pub fn new(engine: Engine) -> Self {
        Self {
            rendering_info: None,
            renderer: None,
            engine,
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
                if let Some(renderer) = &mut self.renderer {
                    let context = &self.rendering_info.as_ref().unwrap().context;
                    let gpu_mesh = raw_mesh_to_gpu_mesh(
                        cube_mesh([0.0, 0.0, -10.0], "".to_string()),
                        renderer,
                        context,
                    )
                    .unwrap();

                    let Some(mut frame_info) = self.engine.return_renderable() else {
                        println!("No active camera, not rendering");
                        return;
                    };

                    frame_info.meshes.push(gpu_mesh);

                    renderer.render(frame_info).unwrap();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}
}

pub fn run(engine: Engine) -> Result<()> {
    let mut app = App::new(engine);
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
