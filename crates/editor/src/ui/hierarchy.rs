use anyhow::Result;
use engine_core::{
    ecs::{
        commands::Commands,
        components::engine_components::{camera::EditorCamera, name::Name},
        entities::Entity,
        query::{filter::Without, query::Query},
        systems::param::ResMut,
    },
    egui::{self, Color32, Frame, Margin, Sense, Stroke, Ui, Vec2, Window},
    rendering::egui::{
        context::EguiContext,
        renderer::{DARK_BG, DIV_COL, HEADER_BG, HOVER_BG, PANEL_BG, TEXT_COL},
    },
    update,
};

const HEADER_TEXT: Color32 = Color32::from_rgb(200, 200, 200);
const ROW_HEIGHT: f32 = 28.0;
const ICON_COL: Color32 = Color32::from_rgb(140, 160, 180);

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

fn centered_text(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    let galley = text_galley(ui, text, size);
    let size = Vec2::new(ui.available_width(), galley.size().y);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let pos = rect.min + (rect.size() - galley.size()) / 2.0;
    ui.painter().galley(pos, galley, color);
}

fn entity_icon(ui: &mut Ui, rect: egui::Rect) {
    let center = egui::pos2(rect.min.x + 10.0, rect.center().y);
    let tri = [
        egui::pos2(center.x - 4.0, center.y - 4.0),
        egui::pos2(center.x + 4.0, center.y),
        egui::pos2(center.x - 4.0, center.y + 4.0),
    ];
    ui.painter()
        .add(egui::Shape::convex_polygon(tri.to_vec(), ICON_COL, Stroke::NONE));
}

#[update]
pub fn hierarchy(
    entities: Query<(Entity, &Name), Without<EditorCamera>>,
    context: ResMut<EguiContext>,
    commands: &mut Commands,
) -> Result<()> {
    Window::new("Hierarchy")
        .title_bar(false)
        .resizable(true)
        .frame(
            Frame::new()
                .fill(PANEL_BG)
                .inner_margin(Margin::same(4))
                .stroke(Stroke::new(1.0, DIV_COL)),
        )
        .show(&context.0, |ui| {
            Frame::new()
                .fill(HEADER_BG)
                .inner_margin(Margin::symmetric(0, 8))
                .show(ui, |ui| {
                    centered_text(ui, "HIERARCHY", 16.0, HEADER_TEXT);
                    ui.painter().line_segment(
                        [
                            egui::pos2(ui.cursor().left() + 6.0, ui.cursor().top() + 2.0),
                            egui::pos2(ui.cursor().right() - 6.0, ui.cursor().top() + 2.0),
                        ],
                        Stroke::new(1.0, DIV_COL),
                    );
                });

            ui.add_space(4.0);

            let btn_width = ui.available_width();
            let spawn_btn = egui::Button::new(
                egui::RichText::new("+ Spawn Entity")
                    .color(TEXT_COL)
                    .size(13.0),
            )
            .fill(DARK_BG)
            .stroke(Stroke::new(1.0, DIV_COL))
            .min_size(Vec2::new(btn_width, 26.0))
            .corner_radius(3.0);

            if ui.add(spawn_btn).clicked() {
                commands.spawn();
            }

            ui.add_space(4.0);

            ui.painter().line_segment(
                [
                    egui::pos2(ui.cursor().left() + 2.0, ui.cursor().top()),
                    egui::pos2(ui.cursor().right() - 2.0, ui.cursor().top()),
                ],
                Stroke::new(1.0, DIV_COL),
            );
            ui.add_space(4.0);

            let mut delete_entity = None;

            for (entity, name) in entities.iter() {
                let available_width = ui.available_width();
                let row_rect = ui.allocate_exact_size(
                    Vec2::new(available_width, ROW_HEIGHT),
                    Sense::click(),
                );

                let rect = row_rect.0;
                let response = row_rect.1;

                let bg = if response.hovered() {
                    HOVER_BG
                } else {
                    PANEL_BG
                };

                ui.painter()
                    .rect_filled(rect, 3.0, bg);

                entity_icon(ui, rect);

                let text_pos = egui::pos2(rect.min.x + 24.0, rect.center().y - 7.0);
                ui.painter().galley(
                    text_pos,
                    text_galley(ui, &name.0, 13.0),
                    TEXT_COL,
                );

                if response.hovered() {
                    ui.painter().rect_stroke(
                        rect,
                        3.0,
                        Stroke::new(1.0, DIV_COL),
                        egui::StrokeKind::Inside,
                    );
                }

                response.context_menu(|ui| {
                    if ui.button("Delete Entity").clicked() {
                        delete_entity = Some(entity);
                        ui.close();
                    }
                });
            }

            if let Some(entity) = delete_entity {
                commands.despawn(entity);
            }

            ui.allocate_space(ui.available_size());
        });

    Ok(())
}
