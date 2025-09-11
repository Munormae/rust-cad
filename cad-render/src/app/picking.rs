use super::AppState;
use cad_core::{EntityKind, Pt2};

impl AppState {
    /// Поиск ближайшей сущности к точке `world` с допуском `tol_px` (в пикселях).
    pub(crate) fn pick_entity(&self, world: Pt2, rect: egui::Rect, tol_px: f32) -> Option<u64> {
        let mut best: Option<(u64, f32)> = None;

        #[inline]
        fn update_best(best: &mut Option<(u64, f32)>, candidate: (u64, f32)) {
            let replace = match *best {
                None => true,
                Some((_id, bd)) => candidate.1 < bd,
            };
            if replace {
                *best = Some(candidate);
            }
        }

        // Возвращает кандидата (id, расстояние) или None.
        let consider_seg = |id: u64, a: Pt2, b: Pt2| -> Option<(u64, f32)> {
            let pp = project_point_to_segment(world, a, b)?;
            let sp = self.to_screen(pp, rect);
            let sw = self.to_screen(world, rect);
            let d = ((sp.x - sw.x).powi(2) + (sp.y - sw.y).powi(2)).sqrt();
            (d <= tol_px).then_some((id, d))
        };

        for e in &self.doc.entities {
            match &e.kind {
                EntityKind::LineSeg { a, b } => {
                    if let Some(c) = consider_seg(e.id, *a, *b) { update_best(&mut best, c); }
                }
                EntityKind::Polyline { pts, .. } => {
                    for w in pts.windows(2) {
                        if let Some(c) = consider_seg(e.id, w[0], w[1]) { update_best(&mut best, c); }
                    }
                }
                EntityKind::Arc { center, radius, start_angle, end_angle } => {
                    let poly = sample_arc_as_polyline(*center, *radius, *start_angle, *end_angle, 64);
                    for w in poly.windows(2) {
                        if let Some(c) = consider_seg(e.id, w[0], w[1]) { update_best(&mut best, c); }
                    }
                }
                EntityKind::NurbsCurve2D { .. } => {
                    let poly = cad_core::sample_entity_nurbs(e, 128).unwrap_or_default();
                    for w in poly.windows(2) {
                        if let Some(c) = consider_seg(e.id, w[0], w[1]) { update_best(&mut best, c); }
                    }
                }
                // Пик текста — по точке вставки
                EntityKind::Text { pos, .. } => {
                    let sp = self.to_screen(*pos, rect);
                    let sw = self.to_screen(world, rect);
                    let d = ((sp.x - sw.x).powi(2) + (sp.y - sw.y).powi(2)).sqrt();
                    if d <= tol_px {
                        update_best(&mut best, (e.id, d));
                    }
                }
            }
        }
        best.map(|(id, _)| id)
    }

    /// Применить рамку выделения (Window/Crossing)
    pub(crate) fn apply_selection_rect(&mut self, sr: super::SelectionRect, _rect: egui::Rect) {
        let (min, max, crossing) = sr.world_bounds();
        self.selection.clear();

        for e in &self.doc.entities {
            let hit = match &e.kind {
                EntityKind::LineSeg { a, b } => {
                    if crossing {
                        segment_intersects_rect(*a, *b, min, max)
                            || rect_contains_point(min, max, *a)
                            || rect_contains_point(min, max, *b)
                    } else {
                        segment_inside_rect(*a, *b, min, max)
                    }
                }
                EntityKind::Polyline { pts, .. } => {
                    if crossing {
                        pts.windows(2).any(|w| segment_intersects_rect(w[0], w[1], min, max))
                            || pts.iter().any(|p| rect_contains_point(min, max, *p))
                    } else {
                        pts.iter().all(|p| rect_contains_point(min, max, *p))
                    }
                }
                EntityKind::Arc { center, radius, start_angle, end_angle } => {
                    let poly = sample_arc_as_polyline(*center, *radius, *start_angle, *end_angle, 64);
                    if crossing {
                        poly.windows(2).any(|w| segment_intersects_rect(w[0], w[1], min, max))
                            || poly.iter().any(|p| rect_contains_point(min, max, *p))
                    } else {
                        poly.iter().all(|p| rect_contains_point(min, max, *p))
                    }
                }
                EntityKind::NurbsCurve2D { .. } => {
                    let poly = cad_core::sample_entity_nurbs(e, 128).unwrap_or_default();
                    if crossing {
                        poly.windows(2).any(|w| segment_intersects_rect(w[0], w[1], min, max))
                            || poly.iter().any(|p| rect_contains_point(min, max, *p))
                    } else {
                        poly.iter().all(|p| rect_contains_point(min, max, *p))
                    }
                }
                // Текст попадает, если его точка вставки внутри прямоугольника
                EntityKind::Text { pos, .. } => rect_contains_point(min, max, *pos),
            };
            if hit { self.selection.add(e.id); }
        }
    }

    /// Экстенты документа (для Fit)
    pub(crate) fn doc_bounds(&self) -> Option<(Pt2, Pt2)> {
        let mut min = Pt2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Pt2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut any = false;

        let acc = |p: Pt2, min: &mut Pt2, max: &mut Pt2, any: &mut bool| {
            if p.x.is_finite() && p.y.is_finite() {
                if p.x < min.x { min.x = p.x; }
                if p.y < min.y { min.y = p.y; }
                if p.x > max.x { max.x = p.x; }
                if p.y > max.y { max.y = p.y; }
                *any = true;
            }
        };

        for e in &self.doc.entities {
            match &e.kind {
                EntityKind::LineSeg { a, b } => { acc(*a, &mut min, &mut max, &mut any); acc(*b, &mut min, &mut max, &mut any); }
                EntityKind::Polyline { pts, .. } => { for &p in pts { acc(p, &mut min, &mut max, &mut any); } }
                EntityKind::Arc { center, radius, start_angle, end_angle } => {
                    for p in sample_arc_as_polyline(*center, *radius, *start_angle, *end_angle, 64) {
                        acc(p, &mut min, &mut max, &mut any);
                    }
                }
                EntityKind::NurbsCurve2D { .. } => {
                    for p in cad_core::sample_entity_nurbs(e, 256).unwrap_or_default() {
                        acc(p, &mut min, &mut max, &mut any);
                    }
                }
                EntityKind::Text { pos, .. } => { acc(*pos, &mut min, &mut max, &mut any); }
            }
        }

        if any { Some((min, max)) } else { None }
    }
}

// === локальные утилиты ===

#[inline]
fn project_point_to_segment(p: Pt2, a: Pt2, b: Pt2) -> Option<Pt2> {
    let vx = b.x - a.x; let vy = b.y - a.y;
    let len2 = vx*vx + vy*vy;
    if len2 == 0.0 { return Some(a); }
    let t = ((p.x - a.x)*vx + (p.y - a.y)*vy) / len2;
    let t = t.clamp(0.0, 1.0);
    Some(Pt2::new(a.x + vx*t, a.y + vy*t))
}

#[inline]
fn sample_arc_as_polyline(c: Pt2, r: f32, sa: f32, ea: f32, n: usize) -> Vec<Pt2> {
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = sa + (ea - sa) * (i as f32) / (n as f32);
        pts.push(Pt2::new(c.x + r * t.cos(), c.y + r * t.sin()));
    }
    pts
}

#[inline]
fn rect_contains_point(min: Pt2, max: Pt2, p: Pt2) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
}
#[inline]
fn segment_inside_rect(a: Pt2, b: Pt2, min: Pt2, max: Pt2) -> bool {
    rect_contains_point(min, max, a) && rect_contains_point(min, max, b)
}
#[inline]
fn segment_intersects_rect(a: Pt2, b: Pt2, min: Pt2, max: Pt2) -> bool {
    if (a.x.max(b.x) < min.x) || (a.x.min(b.x) > max.x) || (a.y.max(b.y) < min.y) || (a.y.min(b.y) > max.y) {
        return false;
    }
    let r1 = (Pt2 { x: min.x, y: min.y }, Pt2 { x: max.x, y: min.y });
    let r2 = (Pt2 { x: max.x, y: min.y }, Pt2 { x: max.x, y: max.y });
    let r3 = (Pt2 { x: max.x, y: max.y }, Pt2 { x: min.x, y: max.y });
    let r4 = (Pt2 { x: min.x, y: max.y }, Pt2 { x: min.x, y: min.y });
    seg_seg(a, b, r1.0, r1.1) || seg_seg(a, b, r2.0, r2.1) || seg_seg(a, b, r3.0, r3.1) || seg_seg(a, b, r4.0, r4.1)
}
#[inline]
fn seg_seg(a: Pt2, b: Pt2, c: Pt2, d: Pt2) -> bool {
    fn orient(a: Pt2, b: Pt2, c: Pt2) -> f32 { (b.x - a.x)*(c.y - a.y) - (b.y - a.y)*(c.x - a.x) }
    fn on_seg(a: Pt2, b: Pt2, p: Pt2) -> bool {
        p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
    }
    let o1 = orient(a, b, c); let o2 = orient(a, b, d);
    let o3 = orient(c, d, a); let o4 = orient(c, d, b);
    if (o1 == 0.0 && on_seg(a, b, c)) || (o2 == 0.0 && on_seg(a, b, d)) ||
        (o3 == 0.0 && on_seg(c, d, a)) || (o4 == 0.0 && on_seg(c, d, b)) { return true; }
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}
