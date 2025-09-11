use cad_core::{sample_entity_nurbs, snap_to_grid, Camera2D, Document, EntityKind, Pt2};
use egui;

/// Тип привязки
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    End,
    Mid,
    Perp,
}

/// Состояние OSNAP
#[derive(Debug, Clone)]
pub struct Osnap {
    pub enabled: bool,
    pub pixel_radius: f32,             // радиус поиска в пикселях
    pub last: Option<(Pt2, SnapKind)>, // последний сработавший снап для отрисовки маркера
}

impl Default for Osnap {
    fn default() -> Self {
        Self {
            enabled: true,
            pixel_radius: 12.0,
            last: None,
        }
    }
}

/// Применить: если снап активен и есть цель — вернуть снап-точку, иначе снап к сетке.
pub fn apply_osnap_or_grid(
    osnap: &Osnap,
    doc: &Document,
    camera: &Camera2D,
    rect: egui::Rect,
    world_pt: Pt2,
) -> Pt2 {
    if osnap.enabled {
        if let Some((_id, sp, _k)) = compute_osnap(doc, camera, rect, world_pt, osnap.pixel_radius)
        {
            return sp;
        }
    }
    snap_to_grid(world_pt, doc.grid.step)
}

/// Поиск ближайшей osnap-точки и её типа возле `world` в пределах `tol_px`.
/// Возвращает (id сущности, мировая точка, тип-снап).
pub fn compute_osnap(
    doc: &Document,
    camera: &Camera2D,
    rect: egui::Rect,
    world: Pt2,
    tol_px: f32,
) -> Option<(u64, Pt2, SnapKind)> {
    let mut best: Option<(u64, Pt2, SnapKind, f32)> = None;

    #[inline]
    fn update_best(
        best: &mut Option<(u64, Pt2, SnapKind, f32)>,
        cand: Option<(u64, Pt2, SnapKind, f32)>,
    ) {
        if let Some(c) = cand {
            let replace = match *best {
                None => true,
                Some((_, _, _, bd)) => c.3 < bd,
            };
            if replace {
                *best = Some(c);
            }
        }
    }

    // кандидат по произвольной точке
    #[inline]
    fn candidate_for_point(
        camera: &Camera2D,
        rect: egui::Rect,
        id: u64,
        p: Pt2,
        kind: SnapKind,
        world: Pt2,
        tol_px: f32,
    ) -> Option<(u64, Pt2, SnapKind, f32)> {
        let sp = to_screen(camera, rect, p);
        let sw = to_screen(camera, rect, world);
        let d = ((sp.x - sw.x).powi(2) + (sp.y - sw.y).powi(2)).sqrt();
        (d <= tol_px).then_some((id, p, kind, d))
    }

    // кандидат по отрезку (концы, середина, перпендикуляр)
    let consider_seg = |id: u64, a: Pt2, b: Pt2| -> [Option<(u64, Pt2, SnapKind, f32)>; 4] {
        let mid = Pt2 {
            x: (a.x + b.x) * 0.5,
            y: (a.y + b.y) * 0.5,
        };
        let mut perp: Option<(u64, Pt2, SnapKind, f32)> = None;
        if let Some(pp) = project_point_to_segment(world, a, b) {
            perp = candidate_for_point(camera, rect, id, pp, SnapKind::Perp, world, tol_px);
        }
        [
            candidate_for_point(camera, rect, id, a, SnapKind::End, world, tol_px),
            candidate_for_point(camera, rect, id, b, SnapKind::End, world, tol_px),
            candidate_for_point(camera, rect, id, mid, SnapKind::Mid, world, tol_px),
            perp,
        ]
    };

    for e in &doc.entities {
        match &e.kind {
            EntityKind::LineSeg { a, b } => {
                for cand in consider_seg(e.id, *a, *b) {
                    update_best(&mut best, cand);
                }
            }
            EntityKind::Polyline { pts, .. } => {
                for w in pts.windows(2) {
                    for cand in consider_seg(e.id, w[0], w[1]) {
                        update_best(&mut best, cand);
                    }
                }
            }
            EntityKind::Arc {
                center,
                radius,
                start_angle,
                end_angle,
            } => {
                let sa = *start_angle;
                let ea = *end_angle;
                let a = Pt2 {
                    x: center.x + radius * sa.cos(),
                    y: center.y + radius * sa.sin(),
                };
                let b = Pt2 {
                    x: center.x + radius * ea.cos(),
                    y: center.y + radius * ea.sin(),
                };
                for cand in consider_seg(e.id, a, b) {
                    update_best(&mut best, cand);
                }
                let poly = sample_arc_as_polyline(*center, *radius, sa, ea, 64);
                for w in poly.windows(2) {
                    for cand in consider_seg(e.id, w[0], w[1]) {
                        update_best(&mut best, cand);
                    }
                }
            }
            EntityKind::NurbsCurve2D { .. } => {
                let poly = sample_entity_nurbs(e, 128).unwrap_or_default();
                for w in poly.windows(2) {
                    for cand in consider_seg(e.id, w[0], w[1]) {
                        update_best(&mut best, cand);
                    }
                }
            }
            EntityKind::Text { pos, .. } => {
                // Снэп к точке вставки текста как к конечной (End)
                update_best(
                    &mut best,
                    candidate_for_point(camera, rect, e.id, *pos, SnapKind::End, world, tol_px),
                );
            }
        }
    }

    best.map(|(id, p, k, _)| (id, p, k))
}

#[inline]
fn to_screen(cam: &Camera2D, rect: egui::Rect, w: Pt2) -> egui::Pos2 {
    let z = cam.zoom.max(0.01);
    egui::pos2(
        rect.left() + (w.x - cam.pan.x) * z,
        rect.top() + (w.y - cam.pan.y) * z,
    )
}

#[inline]
fn sample_arc_as_polyline(c: Pt2, r: f32, sa: f32, ea: f32, n: usize) -> Vec<Pt2> {
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = sa + (ea - sa) * (i as f32) / (n as f32);
        pts.push(Pt2 {
            x: c.x + r * t.cos(),
            y: c.y + r * t.sin(),
        });
    }
    pts
}

#[inline]
fn project_point_to_segment(p: Pt2, a: Pt2, b: Pt2) -> Option<Pt2> {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let len2 = vx * vx + vy * vy;
    if len2 == 0.0 {
        return Some(a);
    }
    let t = ((p.x - a.x) * vx + (p.y - a.y) * vy) / len2;
    let t = t.clamp(0.0, 1.0);
    Some(Pt2 {
        x: a.x + vx * t,
        y: a.y + vy * t,
    })
}
