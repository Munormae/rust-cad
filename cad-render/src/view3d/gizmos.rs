use crate::view3d::Camera;
use cad_core::model3d::Pt3;
use egui::pos2;
use egui::{Align2, Color32, FontId, Pos2, Stroke};

pub fn draw_origin_axes(cam: &Camera, rect: egui::Rect, painter: &egui::Painter) {
    let axes = [
        (
            Pt3::new(0.0, 0.0, 0.0),
            Pt3::new(1000.0, 0.0, 0.0),
            Color32::RED,
            "X",
        ),
        (
            Pt3::new(0.0, 0.0, 0.0),
            Pt3::new(0.0, 1000.0, 0.0),
            Color32::GREEN,
            "Y",
        ),
        (
            Pt3::new(0.0, 0.0, 0.0),
            Pt3::new(0.0, 0.0, 1000.0),
            Color32::BLUE,
            "Z",
        ),
    ];
    for (a, b, color, label) in axes {
        if let (Some(pa), Some(pb)) = (cam.world_to_screen(rect, a), cam.world_to_screen(rect, b)) {
            painter.line_segment([pa, pb], Stroke { width: 1.5, color });
            let dir = (pb - pa).normalized();
            let left = Pos2::new(
                pb.x - 8.0 * dir.x + 4.0 * dir.y,
                pb.y - 8.0 * dir.y - 4.0 * dir.x,
            );
            let right = Pos2::new(
                pb.x - 8.0 * dir.x - 4.0 * dir.y,
                pb.y - 8.0 * dir.y + 4.0 * dir.x,
            );
            painter.line_segment([left, pb], Stroke { width: 1.5, color });
            painter.line_segment([right, pb], Stroke { width: 1.5, color });
            painter.text(
                pb + 6.0 * dir,
                Align2::LEFT_CENTER,
                label,
                FontId::monospace(10.0),
                color,
            );
        }
    }
}

/// Видимый визир pivot (кольцо + крест).
pub fn draw_pivot(cam: &Camera, rect: egui::Rect, painter: &egui::Painter) {
    if let Some(p) = cam.world_to_screen(rect, cam.pivot) {
        let col = Color32::from_rgb(255, 210, 0);
        let stroke = Stroke {
            width: 1.5,
            color: col,
        };
        painter.circle_stroke(p, 8.0, stroke);
        painter.line_segment([pos2(p.x - 6.0, p.y), pos2(p.x + 6.0, p.y)], stroke);
        painter.line_segment([pos2(p.x, p.y - 6.0), pos2(p.x, p.y + 6.0)], stroke);
    }
}
