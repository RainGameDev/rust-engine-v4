use nalgebra::{ClosedAddAssign, ClosedMulAssign, Scalar, Vector3};

use egui::emath::Numeric;

pub fn vec3_drag<T>(input: &mut Vector3<T>, ui: &mut egui::Ui, name: String)
where
    T: Scalar + Copy + ClosedAddAssign + ClosedMulAssign + Numeric,
{
    ui.horizontal(|ui| {
        ui.label(name);
        ui.add(egui::DragValue::new(&mut input.x));
        ui.add(egui::DragValue::new(&mut input.y));
        ui.add(egui::DragValue::new(&mut input.z));
    });
}
