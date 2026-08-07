use std::{
    path::Path,
    time::{Duration, Instant},
};

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
        World,
        components::engine_components::{model_renderer::ModelRenderer, transform::Transform},
        query::query::Query,
        systems::{StartSystem, run_system, scheduler::Schedule},
    },
    input::InputManager,
    log_error, log_warn,
    networking::{client::NetworkClient, snapshot::Snapshot},
    rendering::{
        core::{frame_info::DrawInfo, model::GpuMesh},
        egui::context::EguiContext,
    },
    utils::directory_check::load_directory,
};
use crate::{
    assets::models::{animation::SkinnedMesh, gltf::load_gltf_file},
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
    network: Option<NetworkClient>,
    last_frame: Instant,
    auto_connect_addr: Option<std::net::SocketAddr>,
}

impl App {
    pub fn new(engine: Engine, auto_connect_addr: Option<std::net::SocketAddr>) -> Self {
        Self {
            rendering_info: None,
            renderer: None,
            engine,
            schedule: Schedule::new(60.0),
            network: None,
            last_frame: Instant::now(),
            auto_connect_addr,
        }
    }
    pub fn connect_to_server(&mut self, addr: std::net::SocketAddr) -> Result<()> {
        self.network = Some(NetworkClient::connect(addr)?);
        Ok(())
    }

    fn tick_network(&mut self, delta: Duration) -> Result<()> {
        let Some(network) = &mut self.network else {
            return Ok(());
        };

        network.client.update(delta);
        network.transport.update(delta, &mut network.client)?;

        if network.client.is_connected() {
            while let Some(message) = network.client.receive_message(0) {
                apply_snapshot(&mut self.engine.ecs_world, &message); // game-layer function
            }
        }

        network.transport.send_packets(&mut network.client);
        Ok(())
    }
}
pub fn apply_snapshot(world: &mut World, message: &[u8]) {
    let snapshot: Snapshot = bincode::deserialize(message).unwrap_or_default();
    for entity_state in snapshot.entities {
        // find-or-spawn local entity by NetworkId, write Transform/Health/etc.
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.listen_device_events(winit::event_loop::DeviceEvents::Always);
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
                Ok(loaded) => {
                    for mesh in loaded.meshes {
                        let handle = self.engine.ecs_world.add_asset(mesh, model_path.clone());
                        crate::log_info!("loaded mesh: {} -> {:?}", model_path, handle);
                    }
                    for skeleton in loaded.skeletons {
                        let handle = self
                            .engine
                            .ecs_world
                            .add_asset(skeleton, model_path.clone());
                        crate::log_info!("loaded skeleton: {} -> {:?}", model_path, handle);
                    }
                    for clip in loaded.animations {
                        let handle = self.engine.ecs_world.add_asset(clip, model_path.clone());
                        crate::log_info!("loaded animation: {} -> {:?}", model_path, handle);
                    }
                }
                Err(err) => {
                    crate::log_error!(reason: "failed to load gltf", "{}: {err:?}", model_path);
                }
            }
        }

        let context = EguiContext(self.renderer.as_ref().unwrap().get_egui_context());
        self.engine.ecs_world.add_resource(context);

        let cube = self.engine.ecs_world.spawn();
        self.engine
            .ecs_world
            .add_component(cube, Transform::from_position(Vector3::new(0.0, 0.0, 0.0)));

        let cube_asset = self
            .engine
            .ecs_world
            .get_asset_handle::<GpuMesh>("meshes/cube.glb")
            .unwrap();
        self.engine
            .ecs_world
            .add_component(cube, ModelRenderer { model: cube_asset });

        if let Some(addr) = self.auto_connect_addr {
            match self.connect_to_server(addr) {
                Ok(()) => crate::log_info!("connecting to {addr}..."),
                Err(err) => crate::log_error!(reason: "failed to start connection", "{err:?}"),
            }
        }
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

        if let Ok(mut input) = self.engine.ecs_world.get_resource_mut::<InputManager>() {
            input.process_window_event(&event);
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
                let now = Instant::now();
                let frame_delta = now.duration_since(self.last_frame);
                self.last_frame = now;

                let mut renderer = match self.renderer.take() {
                    Some(r) => r,
                    None => return,
                };

                renderer.begin_ui();

                self.tick_network(frame_delta).unwrap();
                self.schedule.tick(&mut self.engine.ecs_world).unwrap();

                if let Ok(mut input) = self.engine.ecs_world.get_resource_mut::<InputManager>() {
                    input.update();
                }

                let Some(mut frame_info) = self.engine.return_renderable() else {
                    log_error!("No active camera, not rendering");
                    self.renderer = Some(renderer);
                    return;
                };

                let query: Query<(&Transform, &ModelRenderer, Option<&SkinnedMesh>)> =
                    Query::new(&self.engine.ecs_world);

                for (transform, model_renderer, skinned) in query.iter() {
                    let Some(mesh) = self.engine.ecs_world.get_asset(model_renderer.model) else {
                        log_warn!(reason: "stale or missing mesh handle", "skipping entity");
                        continue;
                    };

                    frame_info.draws.push(DrawInfo {
                        mesh: mesh.clone(),
                        model: transform.to_matrix(),
                        joint_matrices: skinned.map(|s| s.joint_matrices.clone()),
                    });
                }

                renderer.render(frame_info).unwrap();
                renderer.end_ui().unwrap();

                self.renderer = Some(renderer);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(info) = &self.rendering_info {
            info.window.request_redraw();
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Ok(mut input) = self.engine.ecs_world.get_resource_mut::<InputManager>() {
            input.process_device_event(&event);
        }
    }
}

pub fn run(engine: Engine, auto_connect_addr: Option<std::net::SocketAddr>) -> Result<()> {
    let mut app = App::new(engine, auto_connect_addr);
    run_system(&mut app.engine.ecs_world, StartSystem::sorted())?;
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
