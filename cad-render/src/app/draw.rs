use super::AppState;
use cad_core::{EntityKind, Pt2, Entity};
use egui::{Ui, Align2, FontId, Color32};

impl AppState {
    pub fn draw_grid(&self, ui: &mut Ui, rect: egui::Rect) {
        if !self.doc.grid.show { return; }
        let step = self.doc.grid.step.max(1.0);

        let wmin = self.from_screen(rect.min, rect);
        let wmax = self.from_screen(rect.max, rect);

        let mut xw = (wmin.x / step).floor() * step;
        let mut yw = (wmin.y / step).floor() * step;
        let major = step * 10.0;

        let color_minor = ui.visuals().weak_text_color();
        let color_major = ui.visuals().widgets.noninteractive.fg_stroke.color;
        let stroke_minor = egui::Stroke { width: 1.0, color: color_minor };
        let stroke_major = egui::Stroke { width: 1.0, color: color_major };

        let mut shapes = Vec::new();
        while yw <= wmax.y {
            let p1 = self.to_screen(Pt2::new(wmin.x, yw), rect);
            let p2 = self.to_screen(Pt2::new(wmax.x, yw), rect);
            let stroke = if (yw / major).abs().fract() == 0.0 { stroke_major } else { stroke_minor };
            shapes.push(egui::Shape::line_segment([p1, p2], stroke));
            yw += step;
        }
        while xw <= wmax.x {
            let p1 = self.to_screen(Pt2::new(xw, wmin.y), rect);
            let p2 = self.to_screen(Pt2::new(xw, wmax.y), rect);
            let stroke = if (xw / major).abs().fract() == 0.0 { stroke_major } else { stroke_minor };
            shapes.push(egui::Shape::line_segment([p1, p2], stroke));
            xw += step;
        }
        ui.painter().extend(shapes);
    }

    pub fn draw_entities(&self, ui: &mut Ui, rect: egui::Rect) {
        for e in &self.doc.entities {
            let selected = self.selection.ids.contains(&e.id);
            let (stroke, text_color) = self.stroke_and_text_color(ui, e, selected);

            match &e.kind {
                EntityKind::LineSeg { a, b } => {
                    ui.painter().line_segment(
                        [self.to_screen(*a, rect), self.to_screen(*b, rect)],
                        stroke,
                    );
                }
                EntityKind::Arc { center, radius, start_angle, end_angle } => {
                    let n = 96usize;
                    let mut pts = Vec::with_capacity(n + 1);
                    let (sa, ea) = (*start_angle, *end_angle);
                    for i in 0..=n {
                        let t = sa + (ea - sa) * (i as f32) / (n as f32);
                        let x = center.x + radius * t.cos();
                        let y = center.y + radius * t.sin();
                        pts.push(self.to_screen(Pt2::new(x, y), rect));
                    }
                    ui.painter().add(egui::Shape::line(pts, stroke));
                }
                EntityKind::Polyline { pts, .. } => {
                    if pts.len() >= 2 {
                        let pts2: Vec<_> = pts.iter().map(|p| self.to_screen(*p, rect)).collect();
                        ui.painter().add(egui::Shape::line(pts2, stroke));
                    }
                }
                EntityKind::NurbsCurve2D { .. } => {
                    let poly = cad_core::sample_entity_nurbs(e, 256).unwrap_or_default();
                    if poly.len() >= 2 {
                        let pts2: Vec<_> = poly.iter().map(|p| self.to_screen(*p, rect)).collect();
                        ui.painter().add(egui::Shape::line(pts2, stroke));
                    }
                }
                EntityKind::Text { pos, content, height } => {
                    let sp = self.to_screen(*pos, rect);
                    // экранный текст (масштабируется с зумом для читаемости)
                    let zoom = self.doc.camera.zoom.max(0.01);
                    let font_size = (*height * zoom).max(8.0);
                    ui.painter().text(
                        sp,
                        Align2::CENTER_CENTER,
                        content,
                        FontId::proportional(font_size),
                        text_color,
                    );
                }
            }
        }

        // osnap marker
        if let Some((p, _k)) = &self.osnap.last {
            let sp = self.to_screen(*p, rect);
            let col = Color32::from_rgb(255, 220, 105);
            let s = 5.0;
            ui.painter().extend([
                egui::Shape::line_segment([egui::pos2(sp.x - s, sp.y), egui::pos2(sp.x + s, sp.y)], egui::Stroke { width: 1.0, color: col }),
                egui::Shape::line_segment([egui::pos2(sp.x, sp.y - s), egui::pos2(sp.x, sp.y + s)], egui::Stroke { width: 1.0, color: col }),
            ]);
        }
    }

    pub fn draw_previews(&self, ui: &mut Ui, rect: egui::Rect, p: Pt2) {
        match self.tool {
            super::Tool::Line => {
                if let Some(a) = self.tmp_pts.first().copied() {
                    let p = self.apply_ortho(a, p);
                    let stroke = egui::Stroke { width: 1.0, color: ui.visuals().weak_text_color() };
                    ui.painter().line_segment([self.to_screen(a, rect), self.to_screen(p, rect)], stroke);
                }
            }
            super::Tool::Nurbs => {
                if !self.tmp_pts.is_empty() {
                    let a0 = *self.tmp_pts.last().unwrap_or(&p);
                    let p = self.apply_ortho(a0, p);
                    let mut pts = self.tmp_pts.clone();
                    pts.push(p);
                    let stroke = egui::Stroke { width: 1.0, color: ui.visuals().weak_text_color() };
                    let screen_pts: Vec<_> = pts.iter().map(|q| self.to_screen(*q, rect)).collect();
                    ui.painter().add(egui::Shape::line(screen_pts, stroke));
                }
            }
            _ => {}
        }
    }

    // ---------- helpers ----------

    /// Подбор цвета штриха и текста с учётом слоя и выделения
    fn stroke_and_text_color(&self, ui: &Ui, e: &Entity, selected: bool) -> (egui::Stroke, Color32) {
        let base = self.layer_color(&e.layer, ui);
        let (line_w, col) = if selected {
            (self.doc.style.stroke_px + 0.8, Self::tint(base, 0.35, ui)) // подсветим
        } else {
            (self.doc.style.stroke_px, base)
        };
        (egui::Stroke { width: line_w, color: col }, col)
    }

    /// Детерминированный цвет по имени слоя (без зависимостей)
    fn layer_color(&self, layer: &str, ui: &Ui) -> Color32 {
        if layer == "0" || layer.is_empty() {
            return ui.visuals().strong_text_color();
        }
        let mut h: u32 = 2166136261; // FNV-1a
        for b in layer.as_bytes() {
            h ^= *b as u32;
            h = h.wrapping_mul(16777619);
        }
        // сделаем приятные пастельные цвета
        let r = ((h >> 16) & 0xFF) as u8;
        let g = ((h >> 8) & 0xFF) as u8;
        let b = (h & 0xFF) as u8;
        let (r, g, b) = (Self::pastel(r), Self::pastel(g), Self::pastel(b));
        Color32::from_rgb(r, g, b)
    }

    #[inline]
    fn pastel(x: u8) -> u8 {
        // компрессия в пастельный диапазон: 96..224
        96 + (x as u16 * 128 / 255) as u8
    }

    /// Лёгкая подсветка цвета (в сторону цвета текста UI)
    fn tint(c: Color32, k: f32, ui: &Ui) -> Color32 {
        let t = ui.visuals().hyperlink_color; // яркий читаемый
        let mix = |a: u8, b: u8| -> u8 {
            let a = a as f32; let b = b as f32;
            (a*(1.0-k) + b*k).round().clamp(0.0,255.0) as u8
        };
        Color32::from_rgb(mix(c.r(), t.r()), mix(c.g(), t.g()), mix(c.b(), t.b()))
    }
}
