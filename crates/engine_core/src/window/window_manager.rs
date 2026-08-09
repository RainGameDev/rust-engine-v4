use std::sync::Arc;

use macros::Resource;
use winit::window::{CursorGrabMode, Window};

#[derive(Default, Clone, Copy, Debug)]
pub enum MouseMode {
    #[default]
    Noop,
    LockedVisible,
    LockedInvisible,
    ConfinedVisible,
    ConfinedInvisible,
}

#[derive(Clone, Debug, Resource)]
pub struct WindowManager {
    pub window: Arc<Window>,
    pub mouse_mode: MouseMode,
}

impl WindowManager {
    /// Changes the mouse mode to the one defined.
    pub fn change_mouse_mode(&mut self, mode: MouseMode) {
        match mode {
            MouseMode::Noop => {
                self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                self.window.set_cursor_visible(true);
            }
            MouseMode::LockedVisible => {
                self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                self.window.set_cursor_visible(true);
            }
            MouseMode::LockedInvisible => {
                self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                self.window.set_cursor_visible(false);
            }
            MouseMode::ConfinedVisible => {
                self.window
                    .set_cursor_grab(CursorGrabMode::Confined)
                    .unwrap();
                self.window.set_cursor_visible(true);
            }
            MouseMode::ConfinedInvisible => {
                self.window
                    .set_cursor_grab(CursorGrabMode::Confined)
                    .unwrap();
                self.window.set_cursor_visible(false);
            }
        }
    }
}
