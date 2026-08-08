use anyhow::Result;
use engine_core::ecs::commands::Commands;
use engine_core::ecs::components::engine_components::camera::GameCamera;
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::ecs::entities::Entity;
use engine_core::ecs::query::filter::With;
use engine_core::ecs::query::query::Query;
use engine_core::ecs::systems::param::{Res, ResMut};
use engine_core::input::InputManager;
use engine_core::input::action::{ActionBinding, InputSource};
use engine_core::nalgebra::{UnitQuaternion, Vector3};
use engine_core::networking::client::NetworkClient;
use engine_core::networking::packet::PlayerMovement;
use engine_core::networking::{INPUT_CHANNEL, Networked};
use engine_core::{Resource, start, update};
use winit::keyboard::{KeyCode, PhysicalKey};

const MOVE_X: &str = "move_x";
const MOVE_Z: &str = "move_z";

const MIN_ZOOM: f32 = 2.0;
const MAX_ZOOM: f32 = 60.0;
const MIN_PITCH: f32 = -1.5;
const MAX_PITCH: f32 = 1.5;

#[derive(Debug, Default, Resource)]
pub struct LastSentDirection(pub [f32; 3]);

#[start]
pub fn bind_input(commands: &mut Commands, mut input: ResMut<InputManager>) -> Result<()> {
    commands.add_resource(LastSentDirection::default());
    commands.add_resource(CameraState::default());

    use InputSource::Keyboard;
    input.bind_action(
        MOVE_X,
        ActionBinding::axis(
            Keyboard(PhysicalKey::Code(KeyCode::KeyD)),
            Keyboard(PhysicalKey::Code(KeyCode::KeyA)),
        ),
    );
    input.bind_action(
        MOVE_Z,
        ActionBinding::axis(
            Keyboard(PhysicalKey::Code(KeyCode::KeyS)),
            Keyboard(PhysicalKey::Code(KeyCode::KeyW)),
        ),
    );

    Ok(())
}

#[update]
pub fn send_movement(
    input: Res<InputManager>,
    mut network: ResMut<NetworkClient>,
    mut last_direction: ResMut<LastSentDirection>,
) -> Result<()> {
    if !network.is_connected() {
        return Ok(());
    }

    let direction = [input.axis(MOVE_X), 0.0, input.axis(MOVE_Z)];
    if direction == last_direction.0 {
        return Ok(());
    }

    network.send(INPUT_CHANNEL, &PlayerMovement { direction })?;
    last_direction.0 = direction;
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
        .map(|(_, _, t)| t.global_position)
    else {
        return Ok(());
    };

    let (dx, dy) = input.mouse_delta();
    state.yaw -= dx * 0.005;
    state.pitch = (state.pitch + dy * 0.005).clamp(MIN_PITCH, MAX_PITCH);
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
    Ok(())
}
