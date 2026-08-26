use engine_core::input::{
    RegisteredInput,
    action::{ActionBinding, InputSource},
};
use winit::keyboard::{KeyCode, PhysicalKey};

/// The list of every action the game supports, with its default binding.
pub fn registered_inputs() -> Vec<RegisteredInput> {
    use KeyCode as K;
    vec![
        RegisteredInput {
            name: "MoveForward",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::KeyW))),
        },
        RegisteredInput {
            name: "MoveBackward",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::KeyS))),
        },
        RegisteredInput {
            name: "MoveLeft",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::KeyA))),
        },
        RegisteredInput {
            name: "MoveRight",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::KeyD))),
        },
        RegisteredInput {
            name: "Jump",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::Space))),
        },
        RegisteredInput {
            name: "Sprint",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::ShiftLeft))),
        },
        RegisteredInput {
            name: "Crouch",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(
                K::ControlLeft,
            ))),
        },
        RegisteredInput {
            name: "Interact",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::KeyE))),
        },
        RegisteredInput {
            name: "Pause",
            default: ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(K::Escape))),
        },
    ]
}
