use anyhow::Result;
use engine_core::{
    ecs::{commands::Commands, systems::param::ResMut},
    egui::{self, Align, Align2, Color32, Frame, Layout, Margin, Vec2, Window},
    log_info,
    rendering::egui::context::EguiContext,
    update,
    window::Quit,
};
use game::GameState;

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
    Window::new("MainMenu")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 150.0))
        .fixed_size(Vec2::new(250.0, 250.0))
        .frame(Frame {
            inner_margin: Margin::same(4),

            fill: Color32::from_rgba_unmultiplied(20, 20, 20, 128),
            ..Default::default()
        })
        .show(&context.0, |ui| {
            ui.set_min_size(Vec2::new(250.0, 220.0));

            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                let button_size = Vec2::new(ui.available_width(), 40.0);

                if ui
                    .add_sized(button_size, egui::Button::new("Continue"))
                    .clicked()
                {
                    log_info!("Starting game through continue");
                    *game_state = GameState::Playing;
                }

                ui.add_space(20.0);

                if ui
                    .add_sized(button_size, egui::Button::new("New Game"))
                    .clicked()
                {
                    log_info!("Starting game through new game");
                    *game_state = GameState::Playing;
                }

                ui.add_space(20.0);

                ui.add_sized(button_size, egui::Button::new("Settings"))
                    .clicked();

                ui.add_space(20.0);

                if ui
                    .add_sized(button_size, egui::Button::new("Exit"))
                    .clicked()
                {
                    commands.add_resource(Quit("Exiting via main menu".into()));
                }
            });
        });

    Ok(())
}
