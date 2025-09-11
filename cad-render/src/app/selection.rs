use crate::app::AppState;
use cad_core::Pt2;
use egui::{CornerRadius, StrokeKind, Ui};
use std::collections::HashSet;

/// Хранит множество выделенных ID и рисует рамку выделения.
#[derive(Debug, Default, Clone)]
pub struct Selection {
    pub ids: HashSet<u64>,
}

impl Selection {
    #[inline]
    pub fn clear(&mut self) {
        self.ids.clear();
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
    #[inline]
    pub fn add(&mut self, id: u64) {
        self.ids.insert(id);
    }
    #[inline]
    pub fn remove(&mut self, id: u64) {
        self.ids.remove(&id);
    }
    #[inline]
    pub fn toggle(&mut self, id: u64) {
        if !self.ids.insert(id) {
            self.ids.remove(&id);
        }
    }
    /// true, если переданный id Some и он уже внутри множества
    #[inline]
    pub fn contains_any(&self, id: Option<u64>) -> bool {
        id.map(|i| self.ids.contains(&i)).unwrap_or(false)
    }

    /// Оверлей поверх сцены: рисуем рамку выделения (window/crossing).
    pub fn draw_overlay(&self, ui: &mut Ui, rect: egui::Rect, app: &AppState) {
        if let Some(sr) = &app.select_rect {
            let (p0, p1, crossing) = sr.screen_bounds(app, rect);

            let color = if crossing {
                egui::Color32::from_rgb(0, 160, 255) // синяя
            } else {
                egui::Color32::from_rgb(0, 200, 120) // зелёная
            };
            let fill = color.linear_multiply(0.06);
            let stroke = egui::Stroke { width: 1.0, color };

            let r = egui::Rect::from_two_pos(p0, p1);
            ui.painter().rect_filled(r, 0.0, fill);
            if crossing {
                // пунктир для crossing
                let dash = 6.0;
                let gap = 4.0;
                let draw_dashed = |a: egui::Pos2, b: egui::Pos2| {
                    let v = b - a;
                    let len = v.length();
                    if len <= 0.0 {
                        return;
                    }
                    let dir = v / len;
                    let mut t = 0.0;
                    while t < len {
                        let t2 = (t + dash).min(len);
                        let p_start = a + dir * t;
                        let p_end = a + dir * t2;
                        ui.painter().line_segment([p_start, p_end], stroke);
                        t += dash + gap;
                    }
                };
                draw_dashed(r.left_top(), r.right_top());
                draw_dashed(r.right_top(), r.right_bottom());
                draw_dashed(r.right_bottom(), r.left_bottom());
                draw_dashed(r.left_bottom(), r.left_top());
            } else {
                ui.painter()
                    .rect_stroke(r, CornerRadius::ZERO, stroke, StrokeKind::Inside);
            }
        }
    }
}

/// Прямоугольник выделения в МИРОВЫХ координатах.
#[derive(Debug, Clone, Copy)]
pub struct SelectionRect {
    pub start: Pt2,
    pub current: Pt2,
}

impl SelectionRect {
    #[inline]
    pub fn start(p: Pt2) -> Self {
        Self {
            start: p,
            current: p,
        }
    }

    #[inline]
    pub fn update_current(&mut self, p: Pt2) {
        self.current = p;
    }

    /// Мировые границы и флаг crossing (true, если тянем справа-налево).
    #[inline]
    pub fn world_bounds(&self) -> (Pt2, Pt2, bool) {
        let min = Pt2 {
            x: self.start.x.min(self.current.x),
            y: self.start.y.min(self.current.y),
        };
        let max = Pt2 {
            x: self.start.x.max(self.current.x),
            y: self.start.y.max(self.current.y),
        };
        let crossing = self.current.x < self.start.x;
        (min, max, crossing)
    }

    /// Экранные координаты рамки для отрисовки (p0..p1) и признак crossing.
    #[inline]
    pub fn screen_bounds(
        &self,
        app: &AppState,
        rect: egui::Rect,
    ) -> (egui::Pos2, egui::Pos2, bool) {
        let (min, max, crossing) = self.world_bounds();
        (app.to_screen(min, rect), app.to_screen(max, rect), crossing)
    }
}
