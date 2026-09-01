pub mod action;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use anyhow::Result;
use gilrs::EventType;
use serde::{Deserialize, Serialize};
use winit::{
    event::{
        DeviceEvent, ElementState, MouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase,
        WindowEvent,
    },
    keyboard::PhysicalKey,
};

use crate::resource;
use crate::log_warn;

use action::{ActionBinding, GamepadAxis, GamepadButton, InputSource};

const ALL_GAMEPAD_BUTTONS: [gilrs::Button; 19] = [
    gilrs::Button::South,
    gilrs::Button::East,
    gilrs::Button::North,
    gilrs::Button::West,
    gilrs::Button::C,
    gilrs::Button::Z,
    gilrs::Button::LeftTrigger,
    gilrs::Button::LeftTrigger2,
    gilrs::Button::RightTrigger,
    gilrs::Button::RightTrigger2,
    gilrs::Button::Select,
    gilrs::Button::Start,
    gilrs::Button::Mode,
    gilrs::Button::LeftThumb,
    gilrs::Button::RightThumb,
    gilrs::Button::DPadUp,
    gilrs::Button::DPadDown,
    gilrs::Button::DPadLeft,
    gilrs::Button::DPadRight,
];

const ALL_GAMEPAD_AXES: [gilrs::Axis; 8] = [
    gilrs::Axis::LeftStickX,
    gilrs::Axis::LeftStickY,
    gilrs::Axis::LeftZ,
    gilrs::Axis::RightStickX,
    gilrs::Axis::RightStickY,
    gilrs::Axis::RightZ,
    gilrs::Axis::DPadX,
    gilrs::Axis::DPadY,
];

/// Phase of a single touch/finger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

impl From<WinitTouchPhase> for TouchPhase {
    fn from(phase: WinitTouchPhase) -> Self {
        match phase {
            WinitTouchPhase::Started => Self::Started,
            WinitTouchPhase::Moved => Self::Moved,
            WinitTouchPhase::Ended => Self::Ended,
            WinitTouchPhase::Cancelled => Self::Cancelled,
        }
    }
}

/// State of a single touch/finger for the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TouchState {
    pub id: u64,
    pub position: (f32, f32),
    pub delta: (f32, f32),
    pub force: f32,
    pub phase: TouchPhase,
}

/// Input manager resource that determines the current state of inputs.
#[resource]
pub struct InputManager {
    gilrs: Option<Mutex<gilrs::Gilrs>>,

    keyboard_pressed: HashSet<PhysicalKey>,
    keyboard_just_pressed: HashSet<PhysicalKey>,
    keyboard_just_released: HashSet<PhysicalKey>,

    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_just_pressed: HashSet<MouseButton>,
    mouse_buttons_just_released: HashSet<MouseButton>,
    mouse_position: (f32, f32),
    mouse_delta: (f32, f32),
    scroll_delta: f32,

    gamepad_buttons_pressed: HashSet<GamepadButton>,
    gamepad_buttons_just_pressed: HashSet<GamepadButton>,
    gamepad_buttons_just_released: HashSet<GamepadButton>,
    gamepad_axes: HashMap<GamepadAxis, f32>,
    gamepad_axis_just_pressed: HashSet<GamepadAxis>,
    gamepad_axis_just_released: HashSet<GamepadAxis>,

    touches: HashMap<u64, TouchState>,
    touch_just_started: bool,

    actions: HashMap<String, ActionBinding>,

    /// Every action the game has registered, in display order, with its default binding.
    registered: Vec<RegisteredInput>,

    axis_pressed_threshold: f32,
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InputManager {
    pub fn new() -> Self {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(gilrs) => Some(Mutex::new(gilrs)),
            Err(err) => {
                log_warn!("gamepad backend unavailable: {err}");
                None
            }
        };

        Self {
            gilrs,
            keyboard_pressed: HashSet::new(),
            keyboard_just_pressed: HashSet::new(),
            keyboard_just_released: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_buttons_just_pressed: HashSet::new(),
            mouse_buttons_just_released: HashSet::new(),
            mouse_position: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            scroll_delta: 0.0,
            gamepad_buttons_pressed: HashSet::new(),
            gamepad_buttons_just_pressed: HashSet::new(),
            gamepad_buttons_just_released: HashSet::new(),
            gamepad_axes: HashMap::new(),
            gamepad_axis_just_pressed: HashSet::new(),
            gamepad_axis_just_released: HashSet::new(),
            touches: HashMap::new(),
            touch_just_started: false,
            actions: HashMap::new(),
            registered: Vec::new(),
            axis_pressed_threshold: 0.5,
        }
    }

    /// Threshold that an analog source must cross to count as "pressed".
    pub fn set_axis_pressed_threshold(&mut self, threshold: f32) {
        self.axis_pressed_threshold = threshold;
    }

    // --- Frame lifecycle ---

    /// Resets per-frame state (`just_pressed`/`just_released`, deltas)
    pub fn update(&mut self) {
        self.keyboard_just_pressed.clear();
        self.keyboard_just_released.clear();
        self.mouse_buttons_just_pressed.clear();
        self.mouse_buttons_just_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.scroll_delta = 0.0;
        self.touch_just_started = false;

        self.update_gamepads();
    }

    fn update_gamepads(&mut self) {
        let Some(gilrs) = &self.gilrs else {
            self.gamepad_buttons_pressed.clear();
            self.gamepad_buttons_just_pressed.clear();
            self.gamepad_buttons_just_released.clear();
            self.gamepad_axes.clear();
            self.gamepad_axis_just_pressed.clear();
            self.gamepad_axis_just_released.clear();
            return;
        };
        let mut gilrs = gilrs.lock().unwrap();

        // Draining the event queue keeps gilrs' cached state fresh.
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::Connected | EventType::Disconnected => {
                    crate::log_info!(
                        "gamepad {} {}",
                        event.id,
                        if matches!(event.event, EventType::Connected) {
                            "connected"
                        } else {
                            "disconnected"
                        }
                    );
                }
                _ => {}
            }
        }

        let previous_buttons = std::mem::take(&mut self.gamepad_buttons_pressed);
        let previous_axes = std::mem::take(&mut self.gamepad_axes);

        let mut pressed = HashSet::new();
        let mut axes: HashMap<GamepadAxis, f32> = HashMap::new();
        for (_, gamepad) in gilrs.gamepads() {
            for button in ALL_GAMEPAD_BUTTONS {
                if gamepad.is_pressed(button) {
                    pressed.insert(GamepadButton::from(button));
                }
            }
            for axis in ALL_GAMEPAD_AXES {
                let value = gamepad.value(axis);
                let ours = GamepadAxis::from(axis);
                let entry = axes.entry(ours).or_insert(0.0);
                if value.abs() > entry.abs() {
                    *entry = value;
                }
            }
        }
        self.gamepad_buttons_pressed = pressed;
        self.gamepad_axes = axes;

        self.gamepad_buttons_just_pressed = self
            .gamepad_buttons_pressed
            .difference(&previous_buttons)
            .copied()
            .collect();
        self.gamepad_buttons_just_released = previous_buttons
            .difference(&self.gamepad_buttons_pressed)
            .copied()
            .collect();

        self.gamepad_axis_just_pressed = self
            .gamepad_axes
            .iter()
            .filter(|(axis, value)| {
                value.abs() > self.axis_pressed_threshold
                    && previous_axes.get(axis).map(|old| old.abs()).unwrap_or(0.0)
                        <= self.axis_pressed_threshold
            })
            .map(|(axis, _)| *axis)
            .collect();
        self.gamepad_axis_just_released = previous_axes
            .iter()
            .filter(|(axis, old)| {
                old.abs() > self.axis_pressed_threshold
                    && self
                        .gamepad_axes
                        .get(*axis)
                        .map(|value| value.abs())
                        .unwrap_or(0.0)
                        <= self.axis_pressed_threshold
            })
            .map(|(axis, _)| *axis)
            .collect();
    }

    // --- Event processing ---

    /// Feed a winit window event into the input manager.
    pub fn process_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.keyboard_input(event.physical_key, event.state, event.repeat);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse_button_input(*button, *state)
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_moved(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => self.handle_mouse_wheel(*delta),
            WindowEvent::Touch(touch) => self.touch_input(
                touch.id,
                TouchPhase::from(touch.phase),
                (touch.location.x as f32, touch.location.y as f32),
                touch.force.map(|f| f.normalized() as f32).unwrap_or(0.0),
            ),
            _ => {}
        }
    }

    /// Feed a winit device event into the input manager.
    pub fn process_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.mouse_delta.0 += delta.0 as f32;
            self.mouse_delta.1 += delta.1 as f32;
        }
    }

    /// Raw keyboard input.
    pub fn keyboard_input(&mut self, key: PhysicalKey, state: ElementState, repeat: bool) {
        match state {
            ElementState::Pressed => {
                if self.keyboard_pressed.insert(key) && !repeat {
                    self.keyboard_just_pressed.insert(key);
                }
            }
            ElementState::Released => {
                if self.keyboard_pressed.remove(&key) {
                    self.keyboard_just_released.insert(key);
                }
            }
        }
    }

    /// Raw mouse button input.
    pub fn mouse_button_input(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.mouse_buttons_pressed.insert(button) {
                    self.mouse_buttons_just_pressed.insert(button);
                }
            }
            ElementState::Released => {
                if self.mouse_buttons_pressed.remove(&button) {
                    self.mouse_buttons_just_released.insert(button);
                }
            }
        }
    }

    /// Absolute cursor position in window pixels.
    pub fn cursor_moved(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    /// Accumulated mouse wheel scroll for this frame.
    pub fn scroll_input(&mut self, delta: f32) {
        self.scroll_delta += delta;
    }

    /// Raw touch input.
    pub fn touch_input(&mut self, id: u64, phase: TouchPhase, position: (f32, f32), force: f32) {
        match phase {
            TouchPhase::Started => {
                self.touch_just_started = true;
                self.touches.insert(
                    id,
                    TouchState {
                        id,
                        position,
                        delta: (0.0, 0.0),
                        force,
                        phase,
                    },
                );
            }
            TouchPhase::Moved => {
                let previous = self
                    .touches
                    .get(&id)
                    .map(|t| t.position)
                    .unwrap_or(position);
                self.touches.insert(
                    id,
                    TouchState {
                        id,
                        position,
                        delta: (position.0 - previous.0, position.1 - previous.1),
                        force,
                        phase,
                    },
                );
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touches.remove(&id);
            }
        }
    }

    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let y = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(position) => position.y as f32 / 50.0,
        };
        self.scroll_input(y);
    }

    // --- Action bindings ---

    /// Records the list of actions the game supports and seeds their
    /// default bindings for any action not already bound.
    pub fn register_inputs(&mut self, inputs: Vec<RegisteredInput>) {
        for input in &inputs {
            self.actions
                .entry(input.name.to_string())
                .or_insert_with(|| input.default.clone());
        }
        self.registered = inputs;
    }

    /// Every registered action in display order, including ones currently unbound.
    pub fn registered_inputs(&self) -> &[RegisteredInput] {
        &self.registered
    }

    /// Adds a binding for an action, replacing any previous binding.
    pub fn bind_action(&mut self, action: impl Into<String>, binding: ActionBinding) {
        self.actions.insert(action.into(), binding);
    }

    /// Adds a source to the positive side of an action's binding.
    pub fn add_source(&mut self, action: impl Into<String>, source: InputSource) {
        self.actions.entry(action.into()).or_default().bind(source);
    }

    /// Adds a source to the negative side of an action's binding.
    pub fn add_negative_source(&mut self, action: impl Into<String>, source: InputSource) {
        self.actions
            .entry(action.into())
            .or_default()
            .negative
            .push(source);
    }

    /// Removes a single source from an action's binding.
    pub fn remove_binding(&mut self, action: &str, source: InputSource) -> bool {
        match self.actions.get_mut(action) {
            Some(binding) => {
                let before = binding.positive.len() + binding.negative.len();
                binding.unbind(&source);
                binding.positive.len() + binding.negative.len() != before
            }
            None => false,
        }
    }

    /// Replaces an action's binding with a single new source.
    pub fn rebind_action(&mut self, action: &str, source: InputSource) -> bool {
        match self.actions.get_mut(action) {
            Some(binding) => {
                binding.positive.clear();
                binding.negative.clear();
                binding.positive.push(source);
                true
            }
            None => false,
        }
    }

    /// Removes an action and all of its bindings.
    pub fn unbind_action(&mut self, action: &str) -> Option<ActionBinding> {
        self.actions.remove(action)
    }

    pub fn has_action(&self, action: &str) -> bool {
        self.actions.contains_key(action)
    }

    /// The currently stored binding for an action, if any.
    pub fn binding(&self, action: &str) -> Option<&ActionBinding> {
        self.actions.get(action)
    }

    /// Returns the first input source that was pressed this frame, if any.
    /// Useful for capture-style keybinding UIs ("press any key").
    pub fn any_source_just_pressed(&self) -> Option<InputSource> {
        self.keyboard_just_pressed
            .iter()
            .next()
            .map(|key| InputSource::Keyboard(*key))
            .or_else(|| {
                self.mouse_buttons_just_pressed
                    .iter()
                    .next()
                    .map(|button| InputSource::Mouse(*button))
            })
            .or_else(|| {
                self.gamepad_buttons_just_pressed
                    .iter()
                    .next()
                    .map(|button| InputSource::GamepadButton(*button))
            })
            .or_else(|| {
                self.gamepad_axis_just_pressed
                    .iter()
                    .next()
                    .map(|axis| InputSource::GamepadAxis(*axis))
            })
            .or({
                if self.touch_just_started {
                    Some(InputSource::Touch)
                } else if self.scroll_delta > 0.0 {
                    Some(InputSource::MouseWheelUp)
                } else if self.scroll_delta < 0.0 {
                    Some(InputSource::MouseWheelDown)
                } else {
                    None
                }
            })
    }

    pub fn actions(&self) -> impl Iterator<Item = (&String, &ActionBinding)> {
        self.actions.iter()
    }

    /// Serializes all current bindings (for saving to disk).
    pub fn save_bindings(&self) -> Vec<u8> {
        bincode::serialize(&self.actions).unwrap_or_default()
    }

    /// Deserializes bindings previously produced by [`InputManager::save_bindings`].
    pub fn load_bindings(&mut self, bytes: &[u8]) -> Result<()> {
        let actions: HashMap<String, ActionBinding> = bincode::deserialize(bytes)
            .map_err(|err| anyhow::anyhow!("failed to load bindings: {err}"))?;
        self.actions = actions;
        Ok(())
    }

    // --- Action queries ---

    /// Whether the action is currently being held.
    pub fn pressed(&self, action: &str) -> bool {
        self.actions
            .get(action)
            .map(|binding| self.binding_active(binding))
            .unwrap_or(false)
    }

    /// Whether the action was pressed this frame.
    pub fn just_pressed(&self, action: &str) -> bool {
        self.actions
            .get(action)
            .map(|binding| binding.positive.iter().any(|s| self.source_just_pressed(s)))
            .unwrap_or(false)
    }

    /// Whether the action was released this frame.
    pub fn just_released(&self, action: &str) -> bool {
        self.actions
            .get(action)
            .map(|binding| {
                binding
                    .positive
                    .iter()
                    .any(|s| self.source_just_released(s))
            })
            .unwrap_or(false)
    }

    /// Continuous value of an action in `[-1.0, 1.0]`.
    /// Positive sources add negative sources subtract.
    pub fn axis(&self, action: &str) -> f32 {
        let Some(binding) = self.actions.get(action) else {
            return 0.0;
        };
        let positive: f32 = binding.positive.iter().map(|s| self.source_value(s)).sum();
        let negative: f32 = binding.negative.iter().map(|s| self.source_value(s)).sum();
        (positive - negative).clamp(-1.0, 1.0)
    }

    fn binding_active(&self, binding: &ActionBinding) -> bool {
        binding.positive.iter().any(|s| self.source_pressed(s))
            || binding.negative.iter().any(|s| self.source_pressed(s))
    }

    fn source_pressed(&self, source: &InputSource) -> bool {
        match source {
            InputSource::GamepadAxis(_) => {
                self.source_value(source).abs() > self.axis_pressed_threshold
            }
            _ => self.source_value(source) != 0.0,
        }
    }

    fn source_just_pressed(&self, source: &InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keyboard_just_pressed.contains(key),
            InputSource::Mouse(button) => self.mouse_buttons_just_pressed.contains(button),
            InputSource::GamepadButton(button) => {
                self.gamepad_buttons_just_pressed.contains(button)
            }
            InputSource::GamepadAxis(axis) => self.gamepad_axis_just_pressed.contains(axis),
            InputSource::MouseWheelUp => self.scroll_delta > 0.0,
            InputSource::MouseWheelDown => self.scroll_delta < 0.0,
            InputSource::Touch => self.touch_just_started,

            _ => false,
        }
    }

    fn source_just_released(&self, source: &InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keyboard_just_released.contains(key),
            InputSource::Mouse(button) => self.mouse_buttons_just_released.contains(button),
            InputSource::GamepadButton(button) => {
                self.gamepad_buttons_just_released.contains(button)
            }
            InputSource::GamepadAxis(axis) => self.gamepad_axis_just_released.contains(axis),
            InputSource::MouseWheelUp | InputSource::MouseWheelDown | InputSource::Touch => false,
            _ => false,
        }
    }

    fn source_value(&self, source: &InputSource) -> f32 {
        match source {
            InputSource::Keyboard(key) if self.keyboard_pressed.contains(key) => 1.0,
            InputSource::Mouse(button) if self.mouse_buttons_pressed.contains(button) => 1.0,
            InputSource::GamepadButton(button) if self.gamepad_buttons_pressed.contains(button) => {
                1.0
            }
            InputSource::GamepadAxis(axis) => self.gamepad_axes.get(axis).copied().unwrap_or(0.0),
            InputSource::MouseWheelUp if self.scroll_delta > 0.0 => 1.0,
            InputSource::MouseWheelDown if self.scroll_delta < 0.0 => 1.0,
            InputSource::Touch => {
                if self.touches.is_empty() {
                    0.0
                } else {
                    1.0
                }
            }
            _ => 0.0,
        }
    }

    // --- Raw state queries ---

    pub fn key_pressed(&self, key: PhysicalKey) -> bool {
        self.keyboard_pressed.contains(&key)
    }

    pub fn key_just_pressed(&self, key: PhysicalKey) -> bool {
        self.keyboard_just_pressed.contains(&key)
    }

    pub fn key_just_released(&self, key: PhysicalKey) -> bool {
        self.keyboard_just_released.contains(&key)
    }

    pub fn mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }

    pub fn mouse_button_just_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_released.contains(&button)
    }

    /// Absolute cursor position in window pixels.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Relative mouse movement accumulated since the last [`InputManager::update`].
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    /// Mouse wheel scroll accumulated since the last [`InputManager::update`].
    pub fn scroll_delta(&self) -> f32 {
        self.scroll_delta
    }

    pub fn gamepad_button_pressed(&self, button: GamepadButton) -> bool {
        self.gamepad_buttons_pressed.contains(&button)
    }

    pub fn gamepad_button_just_pressed(&self, button: GamepadButton) -> bool {
        self.gamepad_buttons_just_pressed.contains(&button)
    }

    pub fn gamepad_button_just_released(&self, button: GamepadButton) -> bool {
        self.gamepad_buttons_just_released.contains(&button)
    }

    pub fn gamepad_axis(&self, axis: GamepadAxis) -> f32 {
        self.gamepad_axes.get(&axis).copied().unwrap_or(0.0)
    }

    /// Number of currently connected gamepads.
    pub fn connected_gamepads(&self) -> usize {
        self.gilrs
            .as_ref()
            .map(|gilrs| gilrs.lock().unwrap().gamepads().count())
            .unwrap_or(0)
    }

    /// Names of currently connected gamepads.
    pub fn gamepad_names(&self) -> Vec<String> {
        let Some(gilrs) = &self.gilrs else {
            return Vec::new();
        };
        gilrs
            .lock()
            .unwrap()
            .gamepads()
            .map(|(_, gamepad)| gamepad.name().to_string())
            .collect()
    }

    pub fn is_touching(&self) -> bool {
        !self.touches.is_empty()
    }

    pub fn touches(&self) -> impl Iterator<Item = &TouchState> {
        self.touches.values()
    }

    pub fn touch(&self, id: u64) -> Option<&TouchState> {
        self.touches.get(&id)
    }
}

/// A single action known to the game, with its default binding.
#[derive(Debug, Clone)]
pub struct RegisteredInput {
    pub name: &'static str,
    pub default: ActionBinding,
}
