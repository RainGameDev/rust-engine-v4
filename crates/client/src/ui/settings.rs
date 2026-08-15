use anyhow::Result;
use engine_core::{
    Resource,
    ash::vk::{CompareOp, CullModeFlags, Filter, FrontFace, PolygonMode},
    ecs::{
        commands::Commands,
        systems::param::{Res, ResMut},
    },
    egui::{Color32, ComboBox, Frame, Margin, Slider, Ui, Window},
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

#[derive(Resource, Debug)]
pub struct SettingsOpen;

#[derive(Debug, Default)]
pub enum SettingsTab {
    #[default]
    Rendering,
}

#[derive(Resource, Debug, Default)]
pub struct SettingsState {
    pub advanced_on: bool,
    pub tab: SettingsTab,
}

#[update]
pub fn settings_menu(
    commands: &mut Commands,
    context: ResMut<EguiContext>,
    open: Res<SettingsOpen>,
    mut render_settings: ResMut<RenderingSettings>,
    mut state: ResMut<SettingsState>,
) -> Result<()> {
    Window::new("Settings")
        .title_bar(false)
        .frame(Frame {
            inner_margin: Margin::same(4),

            fill: Color32::from_rgba_unmultiplied(20, 20, 20, 255),
            ..Default::default()
        })
        .show(&context.0, |ui| {
            ui.separator();
            ui.checkbox(&mut state.advanced_on, "Advanced Settings");

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Rendering").clicked() {
                    state.tab = SettingsTab::Rendering;
                }
            });

            match state.tab {
                SettingsTab::Rendering => {
                    render_settings_tab(ui, render_settings, state.advanced_on)
                }
                _ => {}
            }

            ui.separator();

            ui.allocate_space(ui.available_size());
        });

    Ok(())
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
