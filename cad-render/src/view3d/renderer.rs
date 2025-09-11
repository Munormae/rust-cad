use crate::view3d::{gizmos, Camera};
use cad_core::model3d::{ElementGeom, Project3D, Pt3};
use egui::{CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui};
use std::collections::HashSet;

pub fn draw_scene(ui: &mut Ui, rect: Rect, cam: &Camera, project: &Project3D) {
    let mut lines: Vec<[Pos2; 2]> = Vec::new();

    for model in &project.models {
        for el in &model.elements {
            match &el.geom {
                ElementGeom::Extrusion { profile, height } => {
                    if profile.len() >= 2 {
                        let t = [el.xform[0][3], el.xform[1][3], el.xform[2][3]];
                        let mut bot: Vec<Pt3> = Vec::with_capacity(profile.len());
                        let mut top: Vec<Pt3> = Vec::with_capacity(profile.len());
                        for p in profile {
                            bot.push(Pt3::new(p.x + t[0], p.y + t[1], 0.0 + t[2]));
                            top.push(Pt3::new(p.x + t[0], p.y + t[1], *height + t[2]));
                        }
                        // нежёстко замыкаем контур (если надо, можно добавить последнюю грань)
                        for w in bot.windows(2) {
                            if let (Some(a), Some(b)) = (
                                cam.world_to_screen(rect, w[0]),
                                cam.world_to_screen(rect, w[1]),
                            ) {
                                lines.push([a, b]);
                            }
                        }
                        for w in top.windows(2) {
                            if let (Some(a), Some(b)) = (
                                cam.world_to_screen(rect, w[0]),
                                cam.world_to_screen(rect, w[1]),
                            ) {
                                lines.push([a, b]);
                            }
                        }
                        let n = bot.len().min(top.len());
                        for i in 0..n {
                            if let (Some(a), Some(b)) = (
                                cam.world_to_screen(rect, bot[i]),
                                cam.world_to_screen(rect, top[i]),
                            ) {
                                lines.push([a, b]);
                            }
                        }
                    }
                }
                ElementGeom::SweepCylinder { path, .. } => {
                    if path.len() >= 2 {
                        for w in path.windows(2) {
                            if let (Some(a), Some(b)) = (
                                cam.world_to_screen(rect, w[0]),
                                cam.world_to_screen(rect, w[1]),
                            ) {
                                lines.push([a, b]);
                            }
                        }
                    }
                }
                ElementGeom::Mesh { positions, indices } => {
                    // берём только трансляцию из xform (как и для Extrusion)
                    let t = [el.xform[0][3], el.xform[1][3], el.xform[2][3]];
                    let mut edge_set: HashSet<(u32, u32)> = HashSet::new();
                    for tri in indices.chunks(3) {
                        if tri.len() < 3 {
                            continue;
                        }
                        let (i0, i1, i2) = (tri[0], tri[1], tri[2]);
                        for (a, b) in [(i0, i1), (i1, i2), (i2, i0)] {
                            let key = if a < b { (a, b) } else { (b, a) };
                            if edge_set.insert(key) {
                                let pa = Pt3::new(
                                    positions[a as usize].x + t[0],
                                    positions[a as usize].y + t[1],
                                    positions[a as usize].z + t[2],
                                );
                                let pb = Pt3::new(
                                    positions[b as usize].x + t[0],
                                    positions[b as usize].y + t[1],
                                    positions[b as usize].z + t[2],
                                );
                                if let (Some(sa), Some(sb)) =
                                    (cam.world_to_screen(rect, pa), cam.world_to_screen(rect, pb))
                                {
                                    lines.push([sa, sb]);
                                }
                            }
                        }
                    }
                }
                ElementGeom::Brep(_) => {}
            }
        }
    }

    let painter = ui.painter_at(rect);
    let stroke = Stroke {
        width: 1.0,
        color: ui.visuals().strong_text_color(),
    };
    for seg in lines {
        painter.line_segment(seg, stroke);
    }

    // рамка вида (тонкая)
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke {
            width: 1.0,
            color: ui.visuals().weak_text_color(),
        },
        StrokeKind::Inside,
    );

    // оси в начале координат
    gizmos::draw_origin_axes(cam, rect, &painter);
    gizmos::draw_pivot(cam, rect, &painter);
}

/// bbox по всем элементам (грубая оценка; Brep игнорим)
pub fn bbox_project(p: &Project3D) -> Option<(Pt3, Pt3)> {
    let mut min = Pt3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Pt3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut any = false;

    let mut acc = |q: Pt3| {
        if q.x.is_finite() && q.y.is_finite() && q.z.is_finite() {
            if q.x < min.x {
                min.x = q.x;
            }
            if q.y < min.y {
                min.y = q.y;
            }
            if q.z < min.z {
                min.z = q.z;
            }
            if q.x > max.x {
                max.x = q.x;
            }
            if q.y > max.y {
                max.y = q.y;
            }
            if q.z > max.z {
                max.z = q.z;
            }
            any = true;
        }
    };

    for m in &p.models {
        for e in &m.elements {
            match &e.geom {
                ElementGeom::Extrusion { profile, height } => {
                    let t = [e.xform[0][3], e.xform[1][3], e.xform[2][3]];
                    for p2 in profile {
                        acc(Pt3::new(p2.x + t[0], p2.y + t[1], 0.0 + t[2]));
                        acc(Pt3::new(p2.x + t[0], p2.y + t[1], *height + t[2]));
                    }
                }
                ElementGeom::SweepCylinder { path, .. } => {
                    for p3 in path {
                        acc(*p3);
                    }
                }
                ElementGeom::Mesh { positions, .. } => {
                    for p3 in positions {
                        acc(*p3);
                    }
                }
                ElementGeom::Brep(_) => {}
            }
        }
    }
    if any {
        Some((min, max))
    } else {
        None
    }
}
