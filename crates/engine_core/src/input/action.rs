use serde::{Deserialize, Serialize};
use winit::{event::MouseButton, keyboard::PhysicalKey};

/// A single hardware source of input.
/// Can be serialized so keybinds can be saved to disk and loaded back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputSource {
    Noop,
    /// Physical key (layout independent, what games should use).
    Keyboard(PhysicalKey),
    /// A mouse button.
    Mouse(MouseButton),
    /// A button on a connected gamepad.
    GamepadButton(GamepadButton),
    /// An analog axis on a connected gamepad.
    GamepadAxis(GamepadAxis),
    /// Mouse wheel scrolled up this frame.
    MouseWheelUp,
    /// Mouse wheel scrolled down this frame.
    MouseWheelDown,
    /// Any active touch on the screen.
    Touch,
}

impl InputSource {
    /// Returns `true` if this source produces a continuous value.
    pub fn is_axis(&self) -> bool {
        matches!(self, InputSource::GamepadAxis(_))
    }

    /// Human readable name for the source, used in keybinding menus.
    pub fn display(&self) -> String {
        match self {
            InputSource::Noop => "None".to_string(),
            InputSource::Keyboard(key) => physical_key_name(*key),
            InputSource::Mouse(button) => format!("Mouse {button:?}"),
            InputSource::GamepadButton(button) => format!("Gamepad {button:?}"),
            InputSource::GamepadAxis(axis) => format!("Gamepad {axis:?}"),
            InputSource::MouseWheelUp => "Mouse Wheel Up".to_string(),
            InputSource::MouseWheelDown => "Mouse Wheel Down".to_string(),
            InputSource::Touch => "Touch".to_string(),
        }
    }
}

/// Formats a [`PhysicalKey`] into a short readable name.
fn physical_key_name(key: PhysicalKey) -> String {
    match key {
        PhysicalKey::Code(code) => {
            let debug = format!("{code:?}");
            if let Some(name) = debug.strip_prefix("Key") {
                name.to_string()
            } else if let Some(digit) = debug.strip_prefix("Digit") {
                digit.to_string()
            } else if let Some(numpad) = debug.strip_prefix("Numpad") {
                format!("Num {numpad}")
            } else {
                debug
            }
        }
        PhysicalKey::Unidentified(_) => "Unknown Key".to_string(),
    }
}

/// Mirrors [`gilrs::Button`] but is serializable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadButton {
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Unknown,
}

impl From<gilrs::Button> for GamepadButton {
    fn from(button: gilrs::Button) -> Self {
        use gilrs::Button as B;
        match button {
            B::South => Self::South,
            B::East => Self::East,
            B::North => Self::North,
            B::West => Self::West,
            B::C => Self::C,
            B::Z => Self::Z,
            B::LeftTrigger => Self::LeftTrigger,
            B::LeftTrigger2 => Self::LeftTrigger2,
            B::RightTrigger => Self::RightTrigger,
            B::RightTrigger2 => Self::RightTrigger2,
            B::Select => Self::Select,
            B::Start => Self::Start,
            B::Mode => Self::Mode,
            B::LeftThumb => Self::LeftThumb,
            B::RightThumb => Self::RightThumb,
            B::DPadUp => Self::DPadUp,
            B::DPadDown => Self::DPadDown,
            B::DPadLeft => Self::DPadLeft,
            B::DPadRight => Self::DPadRight,
            B::Unknown => Self::Unknown,
        }
    }
}

impl From<GamepadButton> for gilrs::Button {
    fn from(button: GamepadButton) -> Self {
        use gilrs::Button as B;
        match button {
            GamepadButton::South => B::South,
            GamepadButton::East => B::East,
            GamepadButton::North => B::North,
            GamepadButton::West => B::West,
            GamepadButton::C => B::C,
            GamepadButton::Z => B::Z,
            GamepadButton::LeftTrigger => B::LeftTrigger,
            GamepadButton::LeftTrigger2 => B::LeftTrigger2,
            GamepadButton::RightTrigger => B::RightTrigger,
            GamepadButton::RightTrigger2 => B::RightTrigger2,
            GamepadButton::Select => B::Select,
            GamepadButton::Start => B::Start,
            GamepadButton::Mode => B::Mode,
            GamepadButton::LeftThumb => B::LeftThumb,
            GamepadButton::RightThumb => B::RightThumb,
            GamepadButton::DPadUp => B::DPadUp,
            GamepadButton::DPadDown => B::DPadDown,
            GamepadButton::DPadLeft => B::DPadLeft,
            GamepadButton::DPadRight => B::DPadRight,
            GamepadButton::Unknown => B::Unknown,
        }
    }
}

/// Mirrors [`gilrs::Axis`] but is serializable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    DPadX,
    DPadY,
    Unknown,
}

impl From<gilrs::Axis> for GamepadAxis {
    fn from(axis: gilrs::Axis) -> Self {
        use gilrs::Axis as A;
        match axis {
            A::LeftStickX => Self::LeftStickX,
            A::LeftStickY => Self::LeftStickY,
            A::LeftZ => Self::LeftZ,
            A::RightStickX => Self::RightStickX,
            A::RightStickY => Self::RightStickY,
            A::RightZ => Self::RightZ,
            A::DPadX => Self::DPadX,
            A::DPadY => Self::DPadY,
            A::Unknown => Self::Unknown,
        }
    }
}

impl From<GamepadAxis> for gilrs::Axis {
    fn from(axis: GamepadAxis) -> Self {
        use gilrs::Axis as A;
        match axis {
            GamepadAxis::LeftStickX => A::LeftStickX,
            GamepadAxis::LeftStickY => A::LeftStickY,
            GamepadAxis::LeftZ => A::LeftZ,
            GamepadAxis::RightStickX => A::RightStickX,
            GamepadAxis::RightStickY => A::RightStickY,
            GamepadAxis::RightZ => A::RightZ,
            GamepadAxis::DPadX => A::DPadX,
            GamepadAxis::DPadY => A::DPadY,
            GamepadAxis::Unknown => A::Unknown,
        }
    }
}

/// A set of input sources that drive a single action.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionBinding {
    pub positive: Vec<InputSource>,
    pub negative: Vec<InputSource>,
}

impl ActionBinding {
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds a single button-style source.
    pub fn button(source: InputSource) -> Self {
        Self {
            positive: vec![source],
            negative: Vec::new(),
        }
    }

    /// Binds an axis with a `positive` and `negative` side.
    pub fn axis(positive: InputSource, negative: InputSource) -> Self {
        Self {
            positive: vec![positive],
            negative: vec![negative],
        }
    }

    /// Adds a source to the positive side of the binding.
    pub fn with_positive(mut self, source: InputSource) -> Self {
        self.positive.push(source);
        self
    }

    /// Adds a source to the negative side of the binding.
    pub fn with_negative(mut self, source: InputSource) -> Self {
        self.negative.push(source);
        self
    }

    /// Adds a source to the positive side of the binding, in place.
    pub fn bind(&mut self, source: InputSource) {
        self.positive.push(source);
    }

    /// Removes a source from both sides of the binding.
    pub fn unbind(&mut self, source: &InputSource) {
        self.positive.retain(|s| s != source);
        self.negative.retain(|s| s != source);
    }

    pub fn is_empty(&self) -> bool {
        self.positive.is_empty() && self.negative.is_empty()
    }
}
