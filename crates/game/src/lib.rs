use anyhow::Result;
use engine_core::ecs::commands::Commands;
use engine_core::ecs::systems::param::{Res, ResMut};
use engine_core::input::action::{ActionBinding, InputSource};
use engine_core::input::InputManager;
use engine_core::networking::INPUT_CHANNEL;
use engine_core::networking::client::NetworkClient;
use engine_core::networking::packet::PlayerMovement;
use engine_core::{Resource, start, update};
use winit::keyboard::{KeyCode, PhysicalKey};

const MOVE_X: &str = "move_x";
const MOVE_Z: &str = "move_z";

#[derive(Debug, Default, Resource)]
pub struct LastSentDirection(pub [f32; 3]);

#[start]
pub fn bind_input(commands: &mut Commands, mut input: ResMut<InputManager>) -> Result<()> {
    commands.add_resource(LastSentDirection::default());

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
