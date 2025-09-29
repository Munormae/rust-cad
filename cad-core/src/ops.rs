// cad-core/src/ops.rs
use crate::{Document, Entity, EntityKind, Pt2};
use anyhow::{anyhow, Result};

// math + truck
use cgmath::Point2;
use cryxtal_geometry::prelude::*; // BSplineCurve, KnotVec, трейты

#[inline]
pub fn snap_to_grid(p: Pt2, step: f32) -> Pt2 {
    if step <= 0.0 {
        return p;
    }
    Pt2 {
        x: (p.x / step).round() * step,
        y: (p.y / step).round() * step,
    }
}

pub fn make_line(doc: &mut Document, a: Pt2, b: Pt2, layer: &str) -> u64 {
    doc.add_entity(Entity {
        id: 0,
        layer: layer.into(),
        kind: EntityKind::LineSeg { a, b },
    })
}

pub fn make_arc(
    doc: &mut Document,
    center: Pt2,
    radius: f32,
    start: f32,
    end: f32,
    layer: &str,
) -> u64 {
    doc.add_entity(Entity {
        id: 0,
        layer: layer.into(),
        kind: EntityKind::Arc {
            center,
            radius,
            start_angle: start,
            end_angle: end,
        },
    })
}

pub fn make_polyline(doc: &mut Document, pts: Vec<Pt2>, closed: bool, layer: &str) -> Result<u64> {
    if pts.len() < 2 {
        return Err(anyhow!("Polyline requires at least 2 points"));
    }
    Ok(doc.add_entity(Entity {
        id: 0,
        layer: layer.into(),
        kind: EntityKind::Polyline { pts, closed },
    }))
}

/// Создать открытый равномерный B-сплайн (веса опциональны; при рендере пока игнорируются)
pub fn make_nurbs_open_uniform(
    doc: &mut Document,
    degree: usize,
    ctrl_pts: Vec<Pt2>,
    weights: Option<Vec<f64>>,
    layer: &str,
) -> Result<u64> {
    if degree < 1 {
        return Err(anyhow!("degree must be >= 1"));
    }
    let n = ctrl_pts.len();
    if n < degree + 1 {
        return Err(anyhow!("ctrl_pts must have at least degree + 1 points"));
    }
    if let Some(w) = &weights {
        if w.len() != n {
            return Err(anyhow!("weights length must match ctrl_pts length"));
        }
    }

    // open-uniform knot vector
    let mut knots = vec![0.0; degree + 1];
    if n > degree + 1 {
        let inner = n - degree - 1;
        for i in 1..=inner {
            knots.push(i as f64 / (inner as f64 + 1.0));
        }
    }
    knots.extend(std::iter::repeat(1.0).take(degree + 1));

    Ok(doc.add_entity(Entity {
        id: 0,
        layer: layer.into(),
        kind: EntityKind::NurbsCurve2D {
            degree,
            knots,
            ctrl_pts,
            weights,
        },
    }))
}

pub fn nurbs_from_polyline(doc: &mut Document, poly: &[Pt2], layer: &str) -> Result<u64> {
    if poly.len() < 3 {
        return Err(anyhow!("Need at least 3 points to build degree-2 BSpline"));
    }
    make_nurbs_open_uniform(doc, 2, poly.to_vec(), None, layer)
}

/// Утилита для добавления текста (опционально)
pub fn make_text(
    doc: &mut Document,
    pos: Pt2,
    content: impl Into<String>,
    height: f32,
    layer: &str,
) -> u64 {
    doc.add_entity(Entity {
        id: 0,
        layer: layer.into(),
        kind: EntityKind::Text {
            pos,
            content: content.into(),
            height,
        },
    })
}

/// Семплируем NURBS/BSpline из Entity в полилинию (веса пока игнорируем).
pub fn sample_entity_nurbs(ent: &Entity, samples: usize) -> Option<Vec<Pt2>> {
    match &ent.kind {
        EntityKind::NurbsCurve2D {
            knots, ctrl_pts, ..
        } => {
            // контрольные точки для truck: Point2<f64>
            let ctrl: Vec<Point2<f64>> = ctrl_pts
                .iter()
                .map(|p| Point2::new(p.x as f64, p.y as f64))
                .collect();

            // узловой вектор и параметрический диапазон
            let kv = KnotVec::from(knots.clone());
            let u0 = *knots.first().unwrap_or(&0.0);
            let u1 = *knots.last().unwrap_or(&1.0);

            // B-spline (веса игнорируем)
            let bs = BSplineCurve::<Point2<f64>>::new(kv, ctrl);

            // семплинг
            let mut out = Vec::with_capacity(samples + 1);
            for i in 0..=samples {
                let t = u0 + (u1 - u0) * (i as f64) / (samples as f64);
                let p = bs.subs(t);
                out.push(Pt2 {
                    x: p.x as f32,
                    y: p.y as f32,
                });
            }
            Some(out)
        }
        _ => None,
    }
}

/// Сдвинуть сущность на (dx, dy) в мировых координатах.
pub fn translate_entity(ent: &mut Entity, dx: f32, dy: f32) {
    fn shift(p: &mut Pt2, dx: f32, dy: f32) {
        p.x += dx;
        p.y += dy;
    }

    match &mut ent.kind {
        EntityKind::LineSeg { a, b } => {
            shift(a, dx, dy);
            shift(b, dx, dy);
        }
        EntityKind::Arc { center, .. } => {
            shift(center, dx, dy);
        }
        EntityKind::Polyline { pts, .. } => {
            for p in pts {
                shift(p, dx, dy);
            }
        }
        EntityKind::NurbsCurve2D { ctrl_pts, .. } => {
            for p in ctrl_pts {
                shift(p, dx, dy);
            }
        }
        EntityKind::Text { pos, .. } => {
            shift(pos, dx, dy);
        } // ← добавлено
    }
}
