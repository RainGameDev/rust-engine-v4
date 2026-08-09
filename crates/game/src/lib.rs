use InputSource::*;
use anyhow::Result;
use engine_core::ecs::commands::Commands;
use engine_core::ecs::components::engine_components::camera::{Camera, GameCamera};
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::ecs::entities::Entity;
use engine_core::ecs::query::filter::With;
use engine_core::ecs::query::query::Query;
use engine_core::ecs::systems::param::{Res, ResMut};
use engine_core::input::InputManager;
use engine_core::input::action::{ActionBinding, InputSource};
use engine_core::nalgebra::{Matrix4, Point3, Point4, UnitQuaternion, Vector3};
use engine_core::networking::client::NetworkClient;
use engine_core::networking::packet::MoveTo;
use engine_core::networking::{INPUT_CHANNEL, Networked};
use engine_core::window::window_manager::{MouseMode, WindowManager};
use engine_core::{Resource, start, update};
use winit::event::MouseButton;

const MIN_ZOOM: f32 = 2.0;
const MAX_ZOOM: f32 = 60.0;
const MIN_PITCH: f32 = -1.5;
const MAX_PITCH: f32 = 1.5;

/// Unprojects the cursor through the active camera and intersects the ray
/// with the `y = 0` ground plane. Returns the world-space walk target.
fn screen_to_ground(
    mouse: (f32, f32),
    view_projection: &Matrix4<f32>,
    camera_position: Vector3<f32>,
    window_size: (u32, u32),
) -> Option<Vector3<f32>> {
    let width = window_size.0.max(1) as f32;
    let height = window_size.1.max(1) as f32;

    // winit origin is top-left, NDC origin is bottom-left.
    let ndc = Point3::new(
        2.0 * mouse.0 / width - 1.0,
        1.0 - 2.0 * mouse.1 / height,
        1.0,
    );

    // Inverse view-projection maps NDC to a point on the far plane.
    let inverse = view_projection.try_inverse()?;
    let far = inverse * Point4::new(ndc.x, ndc.y, ndc.z, 1.0);
    let far = far.xyz() / far.w;

    let origin = camera_position;
    let direction = (far - origin).coords.normalize();

    // Intersect the ray with the plane y = 0: 0 = origin.y + t * direction.y.
    let t = -origin.y / direction.y;
    (t >= 0.0).then(|| origin + direction * t)
}

#[start]
pub fn bind_input(commands: &mut Commands, mut input: ResMut<InputManager>) -> Result<()> {
    commands.add_resource(CameraState::default());

    input.bind_action(
        "RotateCamera",
        ActionBinding::axis(Mouse(MouseButton::Right), Noop),
    );

    Ok(())
}

#[update]
pub fn click_to_move(
    input: Res<InputManager>,
    mut network: ResMut<NetworkClient>,
    window_manager: Res<WindowManager>,
    cameras: Query<(&Camera, &Transform), With<GameCamera>>,
) -> Result<()> {
    if !network.is_connected() || !input.mouse_button_just_pressed(MouseButton::Left) {
        return Ok(());
    }

    let Some((camera, transform)) = cameras.iter().next() else {
        return Ok(());
    };

    let window_size = window_manager.window.inner_size();
    let Some(target) = screen_to_ground(
        input.mouse_position(),
        &camera.view_projection_matrix(transform),
        transform.position,
        (window_size.width, window_size.height),
    ) else {
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
