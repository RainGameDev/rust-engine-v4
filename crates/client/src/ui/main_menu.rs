use anyhow::Result;
use engine_core::{
    ecs::{commands::Commands, systems::param::ResMut},
    egui::{self, Align, Align2, Color32, Frame, Layout, Stroke, Vec2, Window},
    log_info,
    rendering::egui::context::EguiContext,
    update,
    window::Quit,
};
use game::GameState;

use crate::ui::settings::SettingsOpen;

#[update]
pub fn main_menu(
    context: ResMut<EguiContext>,
    mut game_state: ResMut<GameState>,
    commands: &mut Commands,
) -> Result<()> {
    if game_state.is_playing() {
        return Ok(());
    }

    let mut ui = egui::Ui::new(
        context.0.clone(),
        egui::Id::new("main_menu_root"),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(context.0.viewport_rect()),
    );

    egui::CentralPanel::no_frame().show(&mut ui, |ui| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/res/images/ui/main_menu.jpg");
        let bytes = std::fs::read(path).expect("failed to read main_menu.jpg");
        let avail = ui.available_size();

        ui.add(
            egui::Image::from_bytes("bytes://main_menu.jpg", bytes)
                .maintain_aspect_ratio(false)
                .fit_to_exact_size(avail),
        );
    });
    let title = "APOSTASY";
    let gold = Color32::from_rgb(228, 186, 94);
    let font = egui::FontId::proportional(50.0);

    let galley = context.0.fonts_mut(|f| {
        f.layout_job(egui::text::LayoutJob::simple(
            title.to_owned(),
            font,
            Color32::PLACEHOLDER,
            f32::INFINITY,
        ))
    });
    let title_size = galley.size() + Vec2::new(40.0, 40.0);

    Window::new("MainMenuLabel")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, -250.0))
        .fixed_size(title_size)
        .frame(Frame::NONE)
        .show(&context.0, |ui| {
            let (rect, _) = ui.allocate_exact_size(title_size, egui::Sense::hover());
            let center = rect.center();
            let top_left = center - galley.size() / 2.0;

            ui.painter().galley(
                top_left + Vec2::new(3.0, 4.0),
                galley.clone(),
                Color32::from_rgba_unmultiplied(10, 8, 2, 210),
            );

            ui.painter().galley(top_left, galley.clone(), gold);

            let line_y = top_left.y + galley.size().y + 14.0;
            ui.painter().line_segment(
                [
                    egui::pos2(top_left.x + 20.0, line_y),
                    egui::pos2(top_left.x + galley.size().x - 20.0, line_y),
                ],
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(228, 186, 94, 180)),
            );
        });

    Window::new("MainMenu")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 150.0))
        .fixed_size(Vec2::new(250.0, 250.0))
        .frame(Frame::NONE)
        .show(&context.0, |ui| {
            ui.set_min_size(Vec2::new(250.0, 220.0));

            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                let item_size = Vec2::new(ui.available_width(), 48.0);
                let text_shadow = Color32::from_rgba_unmultiplied(10, 8, 2, 210);

                let menu_item = |ui: &mut egui::Ui, label: &str| {
                    let galley = ui.ctx().fonts_mut(|f| {
                        f.layout_job(egui::text::LayoutJob::simple(
                            label.to_owned(),
                            egui::FontId::proportional(30.0),
                            Color32::PLACEHOLDER,
                            f32::INFINITY,
                        ))
                    });

                    let (rect, response) = ui.allocate_exact_size(item_size, egui::Sense::click());
                    let text_pos = rect.center() - galley.size() / 2.0;

                    ui.painter().galley(
                        text_pos + Vec2::new(3.0, 4.5),
                        galley.clone(),
                        text_shadow,
                    );

                    let color = if response.hovered() {
                        Color32::from_rgb(255, 226, 170)
                    } else {
                        gold
                    };
                    ui.painter().galley(text_pos, galley, color);

                    response.clicked()
                };

                if menu_item(ui, "Continue") {
                    log_info!("Starting game through continue");
                    *game_state = GameState::Playing;
                }

                ui.add_space(20.0);

                if menu_item(ui, "New Game") {
                    log_info!("Starting game through new game");
                    *game_state = GameState::Playing;
                }

                ui.add_space(20.0);

                if menu_item(ui, "Settings") {
                    commands.add_resource(SettingsOpen);
                }

                ui.add_space(20.0);

                if menu_item(ui, "Exit") {
                    commands.add_resource(Quit("Exiting via main menu".into()));
                }
            });
        });

    Ok(())
}
