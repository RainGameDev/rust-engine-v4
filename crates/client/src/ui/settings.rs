use std::path::PathBuf;

use anyhow::Result;
use engine_core::{
    Resource,
    ash::vk::{CompareOp, CullModeFlags, Filter, FrontFace, PolygonMode},
    ecs::systems::param::{Res, ResMut},
    egui::{
        self, Button, Color32, ComboBox, Frame, Grid, Margin, RichText, Sense, Slider, Stroke, Ui,
        Vec2, Window,
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

const GOLD: Color32 = Color32::from_rgb(228, 186, 94);
const GOLD_BRIGHT: Color32 = Color32::from_rgb(255, 226, 170);
const GOLD_DIM: Color32 = Color32::from_rgb(150, 124, 74);
const TEXT_SHADOW: Color32 = Color32::from_rgba_unmultiplied_const(10, 8, 2, 210);
const WINDOW_FILL: Color32 = Color32::from_rgba_unmultiplied_const(14, 12, 8, 235);
const PANEL_FILL: Color32 = Color32::from_rgb(22, 19, 13);
const BORDER: Color32 = Color32::from_rgb(120, 100, 60);
const KEY_FILL: Color32 = Color32::from_rgb(30, 26, 18);
const KEY_ACTIVE_FILL: Color32 = Color32::from_rgb(58, 50, 24);
const KEY_ACTIVE_BORDER: Color32 = GOLD;
const TEXT_MUTED: Color32 = GOLD_DIM;
const UI_FONT_SIZE: f32 = 28.0;

fn text_galley(ui: &Ui, text: &str, size: f32) -> std::sync::Arc<egui::Galley> {
    ui.ctx().fonts_mut(|f| {
        f.layout_job(egui::text::LayoutJob::simple(
            text.to_owned(),
            egui::FontId::proportional(size),
            Color32::PLACEHOLDER,
            f32::INFINITY,
        ))
    })
}

fn shadowed_text(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    let galley = text_galley(ui, text, size);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    let pos = rect.min;
    ui.painter()
        .galley(pos + Vec2::new(2.0, 3.0), galley.clone(), TEXT_SHADOW);
    ui.painter().galley(pos, galley, color);
}

fn centered_shadowed_text(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    let galley = text_galley(ui, text, size);
    let size = Vec2::new(ui.available_width(), galley.size().y);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let pos = rect.min + (rect.size() - galley.size()) / 2.0;
    ui.painter()
        .galley(pos + Vec2::new(2.0, 3.0), galley.clone(), TEXT_SHADOW);
    ui.painter().galley(pos, galley, color);
}

fn indented_section(ui: &mut Ui, id_salt: &str, label: &str, add_contents: impl FnOnce(&mut Ui)) {
    ui.label(label);
    ui.indent(id_salt, |ui| {
        let style = ui.style_mut();
        style.override_font_id = Some(egui::FontId::proportional(UI_FONT_SIZE - 2.0));
        add_contents(ui);
    });
}

fn gold_popup_style() -> egui::style::StyleModifier {
    egui::style::StyleModifier::new(|style| {
        style.override_font_id = Some(egui::FontId::proportional(UI_FONT_SIZE - 2.0));
        style.spacing.button_padding = egui::Vec2::new(2.0, 0.0);
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, GOLD_DIM);
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, GOLD);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, GOLD_BRIGHT);
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, GOLD_BRIGHT);
        style.visuals.widgets.hovered.weak_bg_fill =
            Color32::from_rgba_unmultiplied(228, 186, 94, 28);
        style.visuals.widgets.active.weak_bg_fill =
            Color32::from_rgba_unmultiplied(228, 186, 94, 48);
        style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        style.visuals.widgets.active.bg_stroke = Stroke::NONE;
        style.visuals.widgets.open.bg_stroke = Stroke::NONE;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(228, 186, 94, 64);
        style.visuals.selection.stroke = Stroke::new(1.0, GOLD);
        style.visuals.window_fill = WINDOW_FILL;
        style.visuals.window_stroke = Stroke::new(1.0, BORDER);
    })
}

fn tab_button(ui: &mut Ui, text: &str, selected: bool, width: f32) -> Response {
    let galley = text_galley(ui, text, UI_FONT_SIZE);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
    let text_pos = rect.center() - galley.size() / 2.0;

    ui.painter()
        .galley(text_pos + Vec2::new(2.0, 3.0), galley.clone(), TEXT_SHADOW);

    let color = if selected {
        GOLD_BRIGHT
    } else if response.hovered() {
        GOLD
    } else {
        GOLD_DIM
    };
    ui.painter().galley(text_pos, galley, color);

    if selected {
        let y = rect.bottom() - 3.0;
        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x + 6.0, y),
                egui::pos2(rect.max.x - 6.0, y),
            ],
            Stroke::new(2.0, GOLD),
        );
    }

    response
}

fn menu_text_button(ui: &mut Ui, text: &str, width: f32) -> Response {
    let galley = text_galley(ui, text, UI_FONT_SIZE);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
    let text_pos = rect.center() - galley.size() / 2.0;

    ui.painter()
        .galley(text_pos + Vec2::new(2.0, 3.0), galley.clone(), TEXT_SHADOW);

    let color = if response.hovered() {
        GOLD_BRIGHT
    } else {
        GOLD
    };
    ui.painter().galley(text_pos, galley, color);

    response
}

/// Key "cap" button that shows the currently bound source and starts capture on click.
fn key_cap(ui: &mut Ui, text: &str, rebinding: bool, width: f32) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(if rebinding { GOLD_BRIGHT } else { GOLD_DIM }))
            .fill(if rebinding { KEY_ACTIVE_FILL } else { KEY_FILL })
            .stroke(if rebinding {
                Stroke::new(1.5, KEY_ACTIVE_BORDER)
            } else {
                Stroke::new(1.0, BORDER)
            })
            .min_size(Vec2::new(width, 36.0))
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
    context
        .0
        .style_mut_of(engine_core::egui::Theme::Dark, |style| {
            style.interaction.selectable_labels = false;
            style.interaction.multi_widget_text_select = false;
        });
    context
        .0
        .style_mut_of(engine_core::egui::Theme::Light, |style| {
            style.interaction.selectable_labels = false;
            style.interaction.multi_widget_text_select = false;
        });

    Window::new("Settings")
        .title_bar(false)
        .default_width(360.0)
        .frame(
            Frame::new()
                .inner_margin(Margin::same(6))
                .fill(WINDOW_FILL)
                .stroke(Stroke::new(2.0, GOLD)),
        )
        .show(&context.0, |ui| {
            {
                let style = ui.style_mut();
                style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, GOLD_DIM);
                style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, GOLD);
                style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, GOLD_BRIGHT);
                style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, GOLD_BRIGHT);
                style.visuals.widgets.inactive.weak_bg_fill = PANEL_FILL;
                style.visuals.widgets.hovered.weak_bg_fill =
                    Color32::from_rgba_unmultiplied(228, 186, 94, 28);
                style.visuals.widgets.active.weak_bg_fill =
                    Color32::from_rgba_unmultiplied(228, 186, 94, 48);
                style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
                style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, GOLD_DIM);
                style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(228, 186, 94, 64);
                style.visuals.selection.stroke = Stroke::new(1.0, GOLD);
                style.visuals.hyperlink_color = GOLD;
                style.visuals.window_fill = WINDOW_FILL;
                style.visuals.window_stroke = Stroke::new(1.0, BORDER);
                style.override_font_id = Some(egui::FontId::proportional(UI_FONT_SIZE));
            }

            centered_shadowed_text(ui, "SETTINGS", 36.0, GOLD);
            ui.painter().line_segment(
                [
                    egui::pos2(ui.cursor().left() + 4.0, ui.cursor().top() + 4.0),
                    egui::pos2(ui.cursor().right() - 4.0, ui.cursor().top() + 4.0),
                ],
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(228, 186, 94, 180)),
            );
            ui.add_space(8.0);

            Frame::new()
                .fill(PANEL_FILL)
                .corner_radius(4)
                .fill(Color32::TRANSPARENT)
                .inner_margin(Margin::symmetric(6, 2))
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

            ui.add_space(8.0);

            ui.checkbox(
                &mut state.advanced_on,
                RichText::new("Advanced Settings").color(GOLD_DIM),
            );

            ui.add_space(6.0);

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
    shadowed_text(ui, "Keybindings", UI_FONT_SIZE, GOLD);
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
        if menu_text_button(ui, "Save", width).clicked() {
            let path = bindings_path();
            match std::fs::write(&path, input.save_bindings()) {
                Ok(()) => log_info!("Saved bindings to {}", path.display()),
                Err(err) => log_error!(reason: "failed to save bindings", "{err}"),
            }
        }
        if menu_text_button(ui, "Reset to Defaults", width).clicked() {
            let registered: Vec<(String, ActionBinding)> = input
                .registered_inputs()
                .iter()
                .map(|r| (r.name.to_string(), r.default.clone()))
                .collect();
            for (name, binding) in registered {
                input.bind_action(name, binding);
            }
            state.rebinding = None;
        }
    });

    ui.add_space(8.0);

    let actions: Vec<(String, ActionBinding)> = input
        .registered_inputs()
        .iter()
        .map(|registered| {
            let binding = input
                .binding(registered.name)
                .cloned()
                .unwrap_or_else(|| registered.default.clone());
            (registered.name.to_string(), binding)
        })
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
                        Button::new(RichText::new("X").color(GOLD_DIM))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, BORDER))
                            .min_size(Vec2::new(30.0, 36.0))
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
        indented_section(ui, "DepthIndent", "Depth", |ui| {
            ui.horizontal(|ui| {
                ui.label("Depth Test Enabled");
                ui.checkbox(&mut render_settings.depth_settings.depth_test_enabled, "")
            });
            ui.horizontal(|ui| {
                ui.label("Depth Compare Type");
                ComboBox::from_id_salt("depth_compare_op")
                    .popup_style(gold_popup_style())
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
        indented_section(ui, "RasterIndent", "Raster", |ui| {
            ui.horizontal(|ui| {
                ui.label("Polygon Mode");
                ComboBox::from_id_salt("polygon_mode")
                    .popup_style(gold_popup_style())
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
                    .popup_style(gold_popup_style())
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
                    .popup_style(gold_popup_style())
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
    indented_section(ui, "ImageIndent", "Image", |ui| {
        ui.horizontal(|ui| {
            ui.label("Filter Mode");
            ComboBox::from_id_salt("filter_mode")
                .popup_style(gold_popup_style())
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
                    .popup_style(gold_popup_style())
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
