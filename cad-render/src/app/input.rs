use egui::{Ui, Response};
use crate::app::Tool;

/// Детект: сейчас панорамируем?
pub fn is_pan_drag(ui: &Ui, resp: &Response, tool: Tool) -> bool {
    let space = ui.input(|i| i.key_down(egui::Key::Space));
    resp.dragged_by(egui::PointerButton::Middle)
        || resp.dragged_by(egui::PointerButton::Secondary)
        || (tool == Tool::Pan && resp.dragged_by(egui::PointerButton::Primary))
        || (space && resp.dragged())
}
