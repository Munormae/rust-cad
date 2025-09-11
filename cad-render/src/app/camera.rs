use super::AppState;
use cad_core::Pt2;

/// Все преобразования и зум/пан для 2D-камеры.
/// Координаты мира — Y-вверх. Экран — Y-вниз, поэтому инвертируем Y.

impl AppState {
    // === Преобразования ===

    /// Мир → экран
    #[inline]
    pub fn to_screen(&self, world: Pt2, rect: egui::Rect) -> egui::Pos2 {
        let z   = self.doc.camera.zoom.max(0.01);
        let pan = self.doc.camera.pan;
        egui::pos2(
            rect.left()   + (world.x - pan.x) * z,
            rect.bottom() - (world.y - pan.y) * z, // инверсия Y
        )
    }

    /// Экран → мир
    #[inline]
    pub fn from_screen(&self, screen: egui::Pos2, rect: egui::Rect) -> Pt2 {
        let z   = self.doc.camera.zoom.max(0.01);
        let pan = self.doc.camera.pan;
        Pt2::new(
            (screen.x - rect.left())   / z + pan.x,
            (rect.bottom() - screen.y) / z + pan.y, // инверсия Y
        )
    }

    // === Управление камерой ===

    /// Колесо мыши — зум к курсору
    pub fn handle_zoom(&mut self, response: &egui::Response, rect: egui::Rect) {
        let scroll_y = response.ctx.input(|i| i.raw_scroll_delta.y);
        if scroll_y.abs() <= 0.0 { return; }

        if let Some(mouse) = response.hover_pos() {
            let factor   = (scroll_y * 0.0015).exp().clamp(0.25, 4.0);
            self.zoom_by(factor, Some((mouse, rect)));
        }
    }

    /// Масштабировать (опционально вокруг точки курсора)
    pub fn zoom_by(&mut self, factor: f32, pivot: Option<(egui::Pos2, egui::Rect)>) {
        let old_zoom = self.doc.camera.zoom.max(0.01);
        let new_zoom = (old_zoom * factor).clamp(0.02, 500.0);

        if let Some((mouse, rect)) = pivot {
            // зум к курсору с учётом инверсии Y
            let world_at_mouse = self.from_screen(mouse, rect);
            let pan_new = Pt2::new(
                world_at_mouse.x - (mouse.x - rect.left())   / new_zoom,
                world_at_mouse.y - (rect.bottom() - mouse.y) / new_zoom,
            );
            self.doc.camera.pan = pan_new;
        }
        self.doc.camera.zoom = new_zoom;
    }

    /// Сброс 1:1 вокруг центра экрана
    pub fn reset_zoom(&mut self, rect: egui::Rect) {
        let center_world = self.from_screen(rect.center(), rect);
        self.doc.camera.zoom = 1.0;
        self.doc.camera.pan  = Pt2::new(
            center_world.x - rect.width()  / 2.0,
            center_world.y - rect.height() / 2.0,
        );
    }

    /// Fit по всем объектам (учитывает Text — якорь)
    pub fn zoom_to_fit_all(&mut self, rect: egui::Rect) {
        if let Some((min, max)) = self.doc_bounds() {
            let margin = 20.0;
            let w = (max.x - min.x).max(1.0) + margin * 2.0;
            let h = (max.y - min.y).max(1.0) + margin * 2.0;
            let z_x = rect.width()  / w;
            let z_y = rect.height() / h;
            let z   = z_x.min(z_y).clamp(0.02, 500.0);
            self.doc.camera.zoom = z;

            // центрируем
            let cx = (min.x + max.x) * 0.5;
            let cy = (min.y + max.y) * 0.5;
            self.doc.camera.pan = Pt2::new(
                cx - rect.width()  / (2.0 * z),
                cy - rect.height() / (2.0 * z),
            );
        }
    }
}
