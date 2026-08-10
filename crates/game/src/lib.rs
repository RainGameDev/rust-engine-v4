use InputSource::*;
use anyhow::Result;
use engine_core::ecs::commands::Commands;
use engine_core::ecs::components::engine_components::camera::{Camera, GameCamera};
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::ecs::entities::Entity;
use engine_core::ecs::query::filter::With;
use engine_core::ecs::query::query::Query;
use engine_core::ecs::systems::param::{Raycast, Res, ResMut};
use engine_core::egui::Pos2;
use engine_core::input::InputManager;
use engine_core::input::action::{ActionBinding, InputSource};
use engine_core::nalgebra::{UnitQuaternion, Vector3};
use engine_core::networking::client::NetworkClient;
use engine_core::networking::packet::MoveTo;
use engine_core::networking::{INPUT_CHANNEL, Networked};
use engine_core::physics::collider::{Collider, ColliderShape};
use engine_core::physics::raycast::{Ray, unproject};
use engine_core::rendering::egui::context::EguiContext;
use engine_core::tiles::TileMap;
use engine_core::window::window_manager::{MouseMode, WindowManager};
use engine_core::{Resource, egui, start, update};
use winit::event::MouseButton;

const MIN_ZOOM: f32 = 2.0;
const MAX_ZOOM: f32 = 60.0;
const MIN_PITCH: f32 = -1.5;
const MAX_PITCH: f32 = 1.5;

/// The tile map both client and server use. Registered at startup.
#[derive(Debug, Resource)]
pub struct TerrainMap(pub TileMap);

/// What the context menu was opened on. Resolved once at right-click time so the
/// menu can't flicker between targets while the camera or mouse moves.
#[derive(Debug, Clone, Copy)]
pub struct ContextMenuTarget {
    pub player_id: Option<u64>,
    pub tile_center: Option<Vector3<f32>>,
}

#[derive(Debug, Resource)]
pub struct ContextMenuOpen(pub bool, pub Pos2, pub Option<ContextMenuTarget>);

#[start]
pub fn bind_input(commands: &mut Commands, mut input: ResMut<InputManager>) -> Result<()> {
    commands.add_resource(CameraState::default());
    commands.add_resource(TerrainMap(TileMap::load_default()?));
    commands.add_resource(ContextMenuOpen(false, Pos2::new(0.0, 0.0), None));

    input.bind_action(
        "RotateCamera",
        ActionBinding::axis(Mouse(MouseButton::Middle), Noop),
    );

    input.bind_action(
        "ContextMenu",
        ActionBinding::axis(Mouse(MouseButton::Right), Noop),
    );

    Ok(())
}

#[update]
pub fn context_menu(
    input: Res<InputManager>,
    window_manager: Res<WindowManager>,
    context: Res<EguiContext>,
    mut network: ResMut<NetworkClient>,
    mut context_menu: ResMut<ContextMenuOpen>,
    cameras: Query<(&Camera, &Transform, Entity), With<GameCamera>>,
    terrain: Res<TerrainMap>,
    raycast: Raycast,
) -> Result<()> {
    let window_size = window_manager.window.inner_size();
    let width = window_size.width.max(1) as f32;
    let height = window_size.height.max(1) as f32;

    let (mx, my) = input.mouse_position();

    if input.mouse_button_just_pressed(MouseButton::Right) {
        if context_menu.0 {
            context_menu.0 = false;
            context_menu.2 = None;
            return Ok(());
        }

        // Opening: resolve the target once and cache it so it can't flicker.
        context_menu.1 = Pos2::new(mx, my);
        context_menu.2 = resolve_context_target(mx, my, width, height, &cameras, &terrain, &raycast);
        context_menu.0 = context_menu.2.is_some();
        if !context_menu.0 {
            return Ok(());
        }
    }

    if !network.is_connected() || !context_menu.0 {
        return Ok(());
    }

    let Some(target) = context_menu.2 else {
        return Ok(());
    };

    let mut move_target: Option<Vector3<f32>> = None;
    let mut attack_id: Option<u64> = None;
    let mut close_menu = false;

    egui::Window::new("ContextMenu")
        .current_pos(context_menu.1)
        .resizable(false)
        .movable(false)
        .title_bar(false)
        .show(&context.0, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("Choose Option");

                ui.separator();

                ui.vertical(|ui| {
                    if let Some(id) = target.player_id {
                        if ui.button("Attack").clicked() {
                            attack_id = Some(id);
                        }
                    } else if let Some(tile) = target.tile_center
                        && ui.button("Move To").clicked()
                    {
                        move_target = Some(tile);
                    }

                    if ui.button("Cancel").clicked() {
                        close_menu = true;
                    }
                });
            });
        });

    if let Some(tile) = move_target {
        network.send(
            INPUT_CHANNEL,
            &MoveTo {
                target: [tile.x, tile.y, tile.z],
            },
        )?;
        close_menu = true;
    }

    if let Some(id) = attack_id {
        engine_core::log_info!("attack player {id} not yet implemented");
        close_menu = true;
    }

    if close_menu {
        context_menu.0 = false;
        context_menu.2 = None;
    }

    Ok(())
}

/// Casts the click ray once and decides what the context menu should target:
/// another player (Attack) or a walkable tile (Move To).
fn resolve_context_target(
    mx: f32,
    my: f32,
    width: f32,
    height: f32,
    cameras: &Query<(&Camera, &Transform, Entity), With<GameCamera>>,
    terrain: &TerrainMap,
    raycast: &Raycast,
) -> Option<ContextMenuTarget> {
    let (camera, transform, camera_entity) = cameras.iter().next()?;

    let ndc_x = 2.0 * mx / width - 1.0;
    let ndc_y = 1.0 - 2.0 * my / height;

    let inv_view_proj = camera.view_projection_matrix(transform).try_inverse()?;
    let direction = unproject(ndc_x, ndc_y, &inv_view_proj, transform.position);
    let ray = Ray::new(transform.position, direction);

    let hit = raycast.cast_ray(&ray, 1000.0, Some(vec![camera_entity]))?;

    let player_id = raycast
        .entity_of::<Networked>(hit.entity_id)
        .map(|networked| networked.id);

    // Only snap to a tile when the click wasn't on a player.
    let tile_center = if player_id.is_none() {
        let (tx, tz) = terrain.0.tile_coord(hit.point);
        terrain
            .0
            .nearest_walkable(tx, tz)
            .and_then(|(wx, wz)| terrain.0.tile_center(wx, wz))
    } else {
        None
    };

    Some(ContextMenuTarget {
        player_id,
        tile_center,
    })
}

/// Adds a collider to networked player entities so raycasts can hit them.
#[update]
pub fn add_player_colliders(
    commands: &mut Commands,
    players: Query<(Entity, Option<&Collider>), With<Networked>>,
) -> Result<()> {
    for (entity, collider) in players.iter() {
        if collider.is_none() {
            commands.add_component(
                entity,
                Collider::new_static(
                    ColliderShape::Cuboid {
                        size: Vector3::new(0.6, 0.6, 0.6),
                    },
                    Vector3::zeros(),
                ),
            );
        }
    }
    Ok(())
}

#[update]
pub fn click_to_move(
    input: Res<InputManager>,
    mut network: ResMut<NetworkClient>,
    window_manager: Res<WindowManager>,
    cameras: Query<(&Camera, &Transform, Entity), With<GameCamera>>,
    terrain: Res<TerrainMap>,
    raycast: Raycast,
    context_menu: Res<ContextMenuOpen>,
) -> Result<()> {
    if !network.is_connected()
        || context_menu.0
        || !input.mouse_button_just_pressed(MouseButton::Left)
    {
        return Ok(());
    }

    let Some((camera, transform, camera_entity)) = cameras.iter().next() else {
        return Ok(());
    };

    let window_size = window_manager.window.inner_size();
    let width = window_size.width.max(1) as f32;
    let height = window_size.height.max(1) as f32;

    // winit origin is top-left, NDC origin is bottom-left.
    let (mx, my) = input.mouse_position();
    let ndc_x = 2.0 * mx / width - 1.0;
    let ndc_y = 1.0 - 2.0 * my / height;

    // Unproject the click into a world-space ray and cast it at the terrain mesh.
    let Some(inv_view_proj) = camera.view_projection_matrix(transform).try_inverse() else {
        return Ok(());
    };
    let direction = unproject(ndc_x, ndc_y, &inv_view_proj, transform.position);
    let ray = Ray::new(transform.position, direction);

    let Some(hit) = raycast.cast_ray(&ray, 1000.0, Some(vec![camera_entity])) else {
        return Ok(());
    };

    // Snap the click to the center of the nearest walkable tile.
    let (tx, tz) = terrain.0.tile_coord(hit.point);
    let Some((wx, wz)) = terrain.0.nearest_walkable(tx, tz) else {
        return Ok(());
    };
    let Some(target) = terrain.0.tile_center(wx, wz) else {
        return Ok(());
    };

    network.send(
        INPUT_CHANNEL,
        &MoveTo {
            target: [target.x, target.y, target.z],
        },
    )?;
    Ok(())
}

#[update]
pub fn update() -> Result<()> {
    Ok(())
}
pub fn init() {}

#[derive(Debug, Resource)]
pub struct CameraState {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.6,
            distance: 12.0,
        }
    }
}

#[update]
pub fn update_camera(
    input: Res<InputManager>,
    network: Res<NetworkClient>,
    mut window_manager: ResMut<WindowManager>,
    mut state: ResMut<CameraState>,
    players: Query<(Entity, &Networked, &Transform)>,
    cameras: Query<&mut Transform, With<GameCamera>>,
) -> Result<()> {
    if !network.is_connected() {
        return Ok(());
    }

    let Some(player_pos) = players
        .iter()
        .find(|(_, n, _)| n.id == network.client_id())
        .map(|(_, _, t)| t.position)
    else {
        return Ok(());
    };

    state.distance = (state.distance - input.scroll_delta() * 1.0).clamp(MIN_ZOOM, MAX_ZOOM);

    let offset = Vector3::new(
        state.yaw.cos() * state.pitch.cos(),
        state.pitch.sin(),
        state.yaw.sin() * state.pitch.cos(),
    ) * state.distance;

    if let Some(transform) = cameras.iter().next() {
        transform.position = player_pos + offset;
        transform.rotation =
            UnitQuaternion::face_towards(&(transform.position - player_pos), &Vector3::y());
    }

    if !input.pressed("RotateCamera") {
        window_manager.change_mouse_mode(MouseMode::Noop);
        return Ok(());
    }

    window_manager.change_mouse_mode(MouseMode::LockedInvisible);

    let (dx, dy) = input.mouse_delta();
    state.yaw -= dx * 0.005;
    state.pitch = (state.pitch + dy * 0.005).clamp(MIN_PITCH, MAX_PITCH);
    Ok(())
}
