use crate::{Document, Entity, EntityKind, Pt2};
use anyhow::Result;

// dxf 0.6 API
use dxf::entities::{
    Entity as DEntity, EntityType, Line as DLine, LwPolyline as DLwPolyline, Spline as DSpline,
    Text as DText,
};
use dxf::{Drawing, LwPolylineVertex, Point as DPoint};

#[inline]
fn p2(x: f64, y: f64) -> Pt2 {
    Pt2 {
        x: x as f32,
        y: y as f32,
    }
}

#[inline]
fn dpoint(p: Pt2) -> DPoint {
    DPoint {
        x: p.x as f64,
        y: p.y as f64,
        z: 0.0,
    }
}

/// Импорт DXF → наш Document (LINE, LWPOLYLINE, SPLINE, TEXT, MTEXT).
pub fn import_dxf(path: &str) -> Result<Document> {
    let drawing = Drawing::load_file(path)?;
    let mut doc = Document::new();

    for e in drawing.entities() {
        let layer = e.common.layer.clone();
        match &e.specific {
            EntityType::Line(line) => {
                let a = p2(line.p1.x, line.p1.y);
                let b = p2(line.p2.x, line.p2.y);
                doc.add_entity(Entity {
                    id: 0,
                    layer,
                    kind: EntityKind::LineSeg { a, b },
                });
            }
            EntityType::LwPolyline(pl) => {
                let pts: Vec<Pt2> = pl.vertices.iter().map(|v| p2(v.x, v.y)).collect();
                let closed = pl.is_closed();
                if pts.len() >= 2 {
                    doc.add_entity(Entity {
                        id: 0,
                        layer,
                        kind: EntityKind::Polyline { pts, closed },
                    });
                }
            }
            EntityType::Spline(sp) => {
                let degree = sp.degree_of_curve as usize;
                let knots: Vec<f64> = sp.knot_values.clone();
                let ctrl_pts: Vec<Pt2> = sp.control_points.iter().map(|p| p2(p.x, p.y)).collect();
                let weights = if sp.weight_values.is_empty() {
                    None
                } else {
                    Some(sp.weight_values.clone())
                };

                if ctrl_pts.len() >= degree + 1 && !knots.is_empty() {
                    doc.add_entity(Entity {
                        id: 0,
                        layer,
                        kind: EntityKind::NurbsCurve2D {
                            degree,
                            knots,
                            ctrl_pts,
                            weights,
                        },
                    });
                }
            }
            EntityType::Text(t) => {
                let pos = p2(t.location.x, t.location.y);
                let height = if t.text_height > 0.0 {
                    t.text_height as f32
                } else {
                    2.5
                };
                let content = t.value.clone();
                doc.add_entity(Entity {
                    id: 0,
                    layer,
                    kind: EntityKind::Text {
                        pos,
                        content,
                        height,
                    },
                });
            }
            EntityType::MText(mt) => {
                let pos = p2(mt.insertion_point.x, mt.insertion_point.y);
                // ключевое поле: initial_text_height
                let height = if mt.initial_text_height > 0.0 {
                    mt.initial_text_height as f32
                } else {
                    2.5
                };
                let content = mt.text.clone();
                doc.add_entity(Entity {
                    id: 0,
                    layer,
                    kind: EntityKind::Text {
                        pos,
                        content,
                        height,
                    },
                });
            }
            _ => {}
        }
    }

    Ok(doc)
}

/// Экспорт Document → DXF (LINE, LWPOLYLINE, SPLINE, TEXT).
pub fn export_dxf(doc: &Document, path: &str) -> Result<()> {
    let mut drawing = Drawing::new();

    for ent in &doc.entities {
        match &ent.kind {
            EntityKind::LineSeg { a, b } => {
                let dl = DLine::new(dpoint(*a), dpoint(*b));
                let mut de = DEntity::new(EntityType::Line(dl));
                de.common.layer = ent.layer.clone();
                drawing.add_entity(de);
            }
            EntityKind::Polyline { pts, closed } => {
                let mut pl = DLwPolyline::default();
                pl.set_is_closed(*closed);
                pl.vertices = pts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let mut v = LwPolylineVertex::default();
                        v.id = i as i32;
                        v.x = p.x as f64;
                        v.y = p.y as f64;
                        v
                    })
                    .collect();
                let mut de = DEntity::new(EntityType::LwPolyline(pl));
                de.common.layer = ent.layer.clone();
                drawing.add_entity(de);
            }
            EntityKind::NurbsCurve2D {
                degree,
                knots,
                ctrl_pts,
                weights,
            } => {
                let mut sp = DSpline::default();
                sp.degree_of_curve = *degree as i32;
                sp.knot_values = knots.clone();
                sp.control_points = ctrl_pts.iter().map(|p| dpoint(*p)).collect();
                if let Some(w) = weights {
                    sp.weight_values = w.clone();
                    sp.set_is_rational(true);
                } else {
                    sp.weight_values.clear();
                    sp.set_is_rational(false);
                }

                let mut de = DEntity::new(EntityType::Spline(sp));
                de.common.layer = ent.layer.clone();
                drawing.add_entity(de);
            }
            EntityKind::Arc { .. } => {
                // DXF Arc можно добавить позже
            }
            EntityKind::Text {
                pos,
                content,
                height,
            } => {
                let mut t = DText::default();
                t.location = dpoint(*pos);
                t.value = content.clone();
                t.text_height = (*height).max(0.1) as f64;
                let mut de = DEntity::new(EntityType::Text(t));
                de.common.layer = ent.layer.clone();
                drawing.add_entity(de);
            }
        }
    }

    drawing.save_file(path)?;
    Ok(())
}
