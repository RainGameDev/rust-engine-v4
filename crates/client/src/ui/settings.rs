use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use engine_core::{
    Resource,
    ash::vk::{CompareOp, CullModeFlags, Filter, FrontFace, PolygonMode},
    ecs::systems::param::{Res, ResMut},
    egui::{
        Button, Color32, ComboBox, Frame, Grid, Margin, RichText, Slider, Stroke, Ui, Vec2, Window,
    },
    input::{
        InputManager,
        action::{ActionBinding, InputSource},
    },
    log_error, log_info,
    rendering::{
        egui::context::EguiContext,
        rendering_settings::RenderingSettings,
        utils::{
            compare_op_to_string, cull_mode_to_string, filter_to_string, front_face_to_string,
            polygon_mode_to_string,
        },
    },
    update,
};
use winit::keyboard::{KeyCode, PhysicalKey};

use engine_core::egui::Response;

const TAB_BAR_FILL: Color32 = Color32::from_rgb(16, 16, 20);
const TAB_ACTIVE_FILL: Color32 = Color32::from_rgb(52, 96, 168);
const TAB_INACTIVE_FILL: Color32 = Color32::from_rgb(34, 34, 40);
const TAB_ACTIVE_BORDER: Color32 = Color32::from_rgb(120, 160, 220);
const BUTTON_FILL: Color32 = Color32::from_rgb(40, 40, 46);
const BORDER: Color32 = Color32::from_rgb(88, 88, 98);
const KEY_FILL: Color32 = Color32::from_rgb(38, 38, 44);
const KEY_ACTIVE_FILL: Color32 = Color32::from_rgb(58, 50, 24);
const KEY_ACTIVE_BORDER: Color32 = Color32::from_rgb(210, 175, 60);
const TEXT_MUTED: Color32 = Color32::from_gray(170);

fn tab_button(ui: &mut Ui, text: &str, selected: bool, width: f32) -> Response {
    let (fill, stroke, text_color) = if selected {
        (
            TAB_ACTIVE_FILL,
            Stroke::new(1.0, TAB_ACTIVE_BORDER),
            Color32::WHITE,
        )
    } else {
        (TAB_INACTIVE_FILL, Stroke::new(1.0, BORDER), TEXT_MUTED)
    };
    ui.add(
        Button::new(RichText::new(text).color(text_color))
            .fill(fill)
            .stroke(stroke)
            .min_size(Vec2::new(width, 26.0))
            .corner_radius(4),
    )
}

fn bordered_button(ui: &mut Ui, text: &str, width: f32) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(Color32::from_gray(210)))
            .fill(BUTTON_FILL)
            .stroke(Stroke::new(1.0, BORDER))
            .min_size(Vec2::new(width, 26.0))
            .corner_radius(4),
    )
}

/// Key "cap" button that shows the currently bound source and starts capture on click.
fn key_cap(ui: &mut Ui, text: &str, rebinding: bool, width: f32) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(Color32::from_gray(225)))
            .fill(if rebinding { KEY_ACTIVE_FILL } else { KEY_FILL })
            .stroke(if rebinding {
                Stroke::new(1.5, KEY_ACTIVE_BORDER)
            } else {
                Stroke::new(1.0, BORDER)
            })
            .min_size(Vec2::new(width, 24.0))
            .corner_radius(4),
    )
}

#[derive(Resource, Debug)]
pub struct SettingsOpen;

#[derive(Debug, Default, PartialEq)]
pub enum SettingsTab {
    #[default]
    Rendering,
    Inputs,
}

#[derive(Resource, Debug, Default)]
pub struct SettingsState {
    pub advanced_on: bool,
    pub tab: SettingsTab,
    /// Action currently waiting for a new input to be pressed.
    pub rebinding: Option<String>,
    /// Skips capturing the first frame so the click that opened capture isn't used.
    pub rebinding_skip_frame: bool,
}

/// Default keybindings, used both to register on startup and to reset.
pub fn default_bindings() -> HashMap<String, ActionBinding> {
    let mut bindings = HashMap::new();
    bindings.insert(
        "Move Forward".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::KeyW))),
    );
    bindings.insert(
        "Move Backward".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::KeyS))),
    );
    bindings.insert(
        "Move Left".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::KeyA))),
    );
    bindings.insert(
        "Move Right".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::KeyD))),
    );
    bindings.insert(
        "Jump".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::Space))),
    );
    bindings.insert(
        "Sprint".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::ShiftLeft))),
    );
    bindings.insert(
        "Crouch".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(
            KeyCode::ControlLeft,
        ))),
    );
    bindings.insert(
        "Interact".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::KeyE))),
    );
    bindings.insert(
        "Pause".to_string(),
        ActionBinding::button(InputSource::Keyboard(PhysicalKey::Code(KeyCode::Escape))),
    );
    bindings
}

/// File bindings are persisted to. Currently just the working directory.
pub fn bindings_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::new())
        .join("bindings.bin")
}

#[update]
pub fn settings_menu(
    context: ResMut<EguiContext>,
    _open: Res<SettingsOpen>,
    render_settings: ResMut<RenderingSettings>,
    mut input: ResMut<InputManager>,
    mut state: ResMut<SettingsState>,
) -> Result<()> {
    context.0.style_mut_of(engine_core::egui::Theme::Dark, |style| {
        style.interaction.selectable_labels = false;
        style.interaction.multi_widget_text_select = false;
    });
    context.0.style_mut_of(engine_core::egui::Theme::Light, |style| {
        style.interaction.selectable_labels = false;
        style.interaction.multi_widget_text_select = false;
    });

    Window::new("Settings")
        .title_bar(false)
        .default_width(360.0)
        .frame(Frame {
            inner_margin: Margin::same(4),

            fill: Color32::from_rgba_unmultiplied(20, 20, 20, 255),
            ..Default::default()
        })
        .show(&context.0, |ui| {
            ui.separator();
            ui.checkbox(&mut state.advanced_on, "Advanced Settings");

            ui.separator();

            Frame::new()
                .fill(TAB_BAR_FILL)
                .corner_radius(4)
                .inner_margin(Margin::same(3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                        if tab_button(ui, "Rendering", state.tab == SettingsTab::Rendering, width)
                            .clicked()
                        {
                            state.tab = SettingsTab::Rendering;
                        }
                        if tab_button(ui, "Inputs", state.tab == SettingsTab::Inputs, width)
                            .clicked()
                        {
                            state.tab = SettingsTab::Inputs;
                        }
                    });
                });

            match state.tab {
                SettingsTab::Rendering => {
                    render_settings_tab(ui, render_settings, state.advanced_on)
                }
                SettingsTab::Inputs => input_settings_tab(ui, &mut input, &mut state),
            }

            ui.separator();

            ui.allocate_space(ui.available_size());
        });

    Ok(())
}

pub fn input_settings_tab(ui: &mut Ui, input: &mut InputManager, state: &mut SettingsState) {
    ui.add_space(6.0);
    ui.label(
        RichText::new("Keybindings")
            .strong()
            .size(15.0)
            .color(Color32::WHITE),
    );
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
        if bordered_button(ui, "Save", width).clicked() {
            let path = bindings_path();
            match std::fs::write(&path, input.save_bindings()) {
                Ok(()) => log_info!("Saved bindings to {}", path.display()),
                Err(err) => log_error!(reason: "failed to save bindings", "{err}"),
            }
        }
        if bordered_button(ui, "Reset to Defaults", width).clicked() {
            for (name, binding) in default_bindings() {
                input.bind_action(name, binding);
            }
            state.rebinding = None;
        }
    });

    ui.add_space(8.0);

    let actions: Vec<(String, ActionBinding)> = input
        .actions()
        .map(|(name, binding)| (name.clone(), binding.clone()))
        .collect();

    Grid::new("input_binding_grid")
        .num_columns(3)
        .min_col_width(0.0)
        .spacing(Vec2::new(10.0, 8.0))
        .show(ui, |ui| {
            for (name, binding) in &actions {
                let is_rebinding = state.rebinding.as_deref() == Some(name.as_str());

                ui.label(RichText::new(name).color(TEXT_MUTED));

                let source = binding
                    .positive
                    .first()
                    .map(InputSource::display)
                    .unwrap_or_else(|| "Unbound".to_string());
                let key_text = if is_rebinding {
                    "Press a key...".to_string()
                } else {
                    source
                };

                let key_clicked = key_cap(ui, &key_text, is_rebinding, 130.0).clicked();
                let clear_clicked = ui
                    .add(
                        Button::new(RichText::new("X").color(Color32::from_gray(150)))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, BORDER))
                            .min_size(Vec2::new(22.0, 24.0))
                            .corner_radius(4),
                    )
                    .on_hover_text("Unbind")
                    .clicked();

                if key_clicked {
                    state.rebinding = Some(name.clone());
                    state.rebinding_skip_frame = true;
                }
                if clear_clicked {
                    input.bind_action(name.clone(), ActionBinding::new());
                    state.rebinding = None;
                }

                ui.end_row();
            }
        });

    if let Some(action) = state.rebinding.clone() {
        ui.add_space(4.0);
        ui.label(
            RichText::new("Press the key to bind... (Esc to cancel)").color(KEY_ACTIVE_BORDER),
        );
        if state.rebinding_skip_frame {
            state.rebinding_skip_frame = false;
        } else if input.key_just_pressed(PhysicalKey::Code(KeyCode::Escape)) {
            state.rebinding = None;
        } else if let Some(source) = input.any_source_just_pressed() {
            input.rebind_action(&action, source);
            state.rebinding = None;
        }
    }
}

pub fn render_settings_tab(
    ui: &mut Ui,
    mut render_settings: ResMut<RenderingSettings>,
    is_advanced: bool,
) {
    if is_advanced {
        ui.separator();
        ui.label("Depth");

        ui.indent("DepthIndent", |ui| {
            ui.horizontal(|ui| {
                ui.label("Depth Test Enabled");
                ui.checkbox(&mut render_settings.depth_settings.depth_test_enabled, "")
            });
            ui.horizontal(|ui| {
                ui.label("Depth Compare Type");
                ComboBox::from_id_salt("depth_compare_op")
                    .selected_text(compare_op_to_string(
                        render_settings.depth_settings.depth_compare_op,
                    ))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::NEVER,
                            "Never",
                        );
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::LESS,
                            "Less",
                        );
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::EQUAL,
                            "Equal",
                        );
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::LESS_OR_EQUAL,
                            "Less or equal",
                        );
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::GREATER,
                            "Greater",
                        );
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::NOT_EQUAL,
                            "Not equal",
                        );
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::GREATER_OR_EQUAL,
                            "Greater or equal",
                        );
                        ui.selectable_value(
                            &mut render_settings.depth_settings.depth_compare_op,
                            CompareOp::ALWAYS,
                            "Always",
                        );
                    });
            });
        });
    }

    ui.separator();

    if is_advanced {
        ui.label("Raster");

        ui.indent("RasterIndent", |ui| {
            ui.horizontal(|ui| {
                ui.label("Polygon Mode");
                ComboBox::from_id_salt("polygon_mode")
                    .selected_text(polygon_mode_to_string(
                        render_settings.rasterization_settings.polygon_mode,
                    ))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.polygon_mode,
                            PolygonMode::FILL,
                            "Fill",
                        );
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.polygon_mode,
                            PolygonMode::POINT,
                            "Point",
                        );
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.polygon_mode,
                            PolygonMode::LINE,
                            "Line",
                        );
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Cull Mode");
                ComboBox::from_id_salt("cull_mode")
                    .selected_text(cull_mode_to_string(
                        render_settings.rasterization_settings.cull_mode,
                    ))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.cull_mode,
                            CullModeFlags::BACK,
                            "Back",
                        );
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.cull_mode,
                            CullModeFlags::FRONT,
                            "Front",
                        );
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.cull_mode,
                            CullModeFlags::FRONT_AND_BACK,
                            "Front and back",
                        );
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.cull_mode,
                            CullModeFlags::NONE,
                            "None",
                        );
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Front Face");
                ComboBox::from_id_salt("front_face")
                    .selected_text(front_face_to_string(
                        render_settings.rasterization_settings.front_face,
                    ))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.front_face,
                            FrontFace::CLOCKWISE,
                            "Clockwise",
                        );
                        ui.selectable_value(
                            &mut render_settings.rasterization_settings.front_face,
                            FrontFace::COUNTER_CLOCKWISE,
                            "Counter clockwise",
                        );
                    });
            });

            if render_settings.rasterization_settings.polygon_mode == PolygonMode::LINE {
                ui.horizontal(|ui| {
                    ui.label("Line Width");
                    ui.add(Slider::new(
                        &mut render_settings.rasterization_settings.line_width,
                        0.1..=16.0,
                    ));
                });
            }
        });
    }

    ui.separator();
    ui.label("Image");

    ui.indent("ImageIndent", |ui| {
        ui.horizontal(|ui| {
            ui.label("Filter Mode");
            ComboBox::from_id_salt("filter_mode")
                .selected_text(filter_to_string(render_settings.image_settings.filter_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut render_settings.image_settings.filter_mode,
                        Filter::LINEAR,
                        "Linear",
                    );
                    ui.selectable_value(
                        &mut render_settings.image_settings.filter_mode,
                        Filter::NEAREST,
                        "Nearest",
                    );
                });
        });
        ui.horizontal(|ui| {
            ui.label("Anistropy Enabled");
            ui.checkbox(&mut render_settings.image_settings.anisotropy_enabled, "");
        });

        if render_settings.image_settings.anisotropy_enabled {
            ui.horizontal(|ui| {
                ui.label("Anistropy Amount");
                ComboBox::from_id_salt("anisotropy_amount")
                    .selected_text(format!(
                        "{}x",
                        render_settings.image_settings.anisotropy_amount
                    ))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut render_settings.image_settings.anisotropy_amount,
                            2,
                            "2x",
                        );
                        ui.selectable_value(
                            &mut render_settings.image_settings.anisotropy_amount,
                            4,
                            "4x",
                        );
                        ui.selectable_value(
                            &mut render_settings.image_settings.anisotropy_amount,
                            8,
                            "8x",
                        );
                        ui.selectable_value(
                            &mut render_settings.image_settings.anisotropy_amount,
                            16,
                            "16x",
                        );
                    });
            });
        }
    });

    ui.separator();
    ui.label("Debug");
}
