//! Простая «склейка» наших 2D-контуров с truck: Wire → Face → Solid → TriMesh.
//! Держим всё максимально прямолинейно, чтобы быстро показать геометрию.

use truck_geometry::prelude::*;
use truck_modeling::*;
use truck_meshalgo::prelude::*;

use crate::geom::{EntityKind, Pt2 /*, TruckCurve2*/};

fn p2(p: Pt2) -> Point2<f64> {
    Point2::new(p.x as f64, p.y as f64)
}
fn p3_xy(p: Pt2, z: f64) -> Point3<f64> {
    Point3::new(p.x as f64, p.y as f64, z)
}

/// Гарантируем замкнутость полилинии (дублируем первую точку в конец при необходимости).
pub fn close_poly_if_needed(poly: &mut Vec<Pt2>) {
    if poly.len() >= 2 {
        if let (Some(a), Some(b)) = (poly.first(), poly.last()) {
            if a != b {
                poly.push(*a);
            }
        }
    }
}

/// Простейший wire из замкнутой полилинии: каждое звено — линейный BSpline степени 1.
pub fn wire_from_closed_polyline(pts: &[Pt2]) -> Wire {
    assert!(pts.len() >= 3, "нужно минимум 3 точки для контура");

    let mut edges = Vec::with_capacity(pts.len().saturating_sub(1));
    for i in 0..pts.len() - 1 {
        let a = p3_xy(pts[i], 0.0);
        let b = p3_xy(pts[i + 1], 0.0);
        let va = Vertex::new(a);
        let vb = Vertex::new(b);

        // Линию кодируем как B-сплайн степени 1 на двух опорных точках.
        // Узловой вектор: [0,0,1,1].
        let curve3d = BSplineCurve::new(KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]), vec![a, b]);
        let curve = Curve::BSplineCurve(Arc::new(curve3d));
        edges.push(Edge::new(&va, &vb, curve));
    }
    Wire::from_iter(edges)
}

/// Плоская грань в плоскости Z=0 по набору проволок.
/// Первая проволока = внешний контур, остальные — отверстия.
pub fn planar_face_from_wires(outer: &Wire, holes: &[Wire]) -> Face {
    let plane = Plane::new(Vector3::unit_z(), Point3::origin());
    let surf = PlaneSurface::new(plane);

    // Face::new(surface, vec![outer, ...holes], true)
    let mut all = Vec::with_capacity(1 + holes.len());
    all.push(outer.clone());
    all.extend(holes.iter().cloned());
    Face::new(surf, all, true)
}

/// Упрощённая версия: грань только из одной проволоки (без отверстий).
pub fn planar_face_from_wire(outer: &Wire) -> Face {
    planar_face_from_wires(outer, &[])
}

/// Экструзия плоской грани на высоту `h` вдоль +Z → Solid.
pub fn extrude_face(face: &Face, h: f64) -> Solid {
    let dir = Vector3::new(0.0, 0.0, h);
    face.sweep(&dir)
}

/// Базовая триангуляция Solid при помощи truck_meshalgo.
pub fn mesh_from_solid(solid: &Solid) -> (Vec<[f32; 3]>, Vec<u32>) {
    let pm: PolygonMesh = solid.triangulation(Default::default());
    let positions: Vec<[f32; 3]> = pm
        .positions()
        .iter()
        .map(|p| [p.x as f32, p.y as f32, p.z as f32])
        .collect();
    let indices: Vec<u32> = pm.indices().iter().map(|&i| i as u32).collect();
    (positions, indices)
}

/// Применить 4×4 матрицу (row-major) к позиции в однородных координатах.
#[inline]
fn transform_point(mat: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    let x = p[0];
    let y = p[1];
    let z = p[2];
    let w = 1.0_f32;

    let xp = mat[0][0] * x + mat[0][1] * y + mat[0][2] * z + mat[0][3] * w;
    let yp = mat[1][0] * x + mat[1][1] * y + mat[1][2] * z + mat[1][3] * w;
    let zp = mat[2][0] * x + mat[2][1] * y + mat[2][2] * z + mat[2][3] * w;
    let wp = mat[3][0] * x + mat[3][1] * y + mat[3][2] * z + mat[3][3] * w;

    if wp != 0.0 {
        [xp / wp, yp / wp, zp / wp]
    } else {
        [xp, yp, zp]
    }
}

/// Применить матрицу к массиву вершин (in-place).
pub fn transform_positions_inplace(positions: &mut [[f32; 3]], mat: &[[f32; 4]; 4]) {
    for p in positions.iter_mut() {
        *p = transform_point(mat, *p);
    }
}

/// Полный пайплайн: внешний контур + отверстия (все в 2D XY, Z=0) → грань → экструзия → триангуляция,
/// далее — применяем матрицу трансформации к вершинам.
pub fn extrude_polygon_to_mesh_with_transform(
    outer: &[Pt2],
    holes: &[Vec<Pt2>],
    height: f64,
    xform_row_major: &[[f32; 4]; 4],
) -> (Vec<[f32; 3]>, Vec<u32>) {
    // 1) Обеспечиваем замкнутость
    let mut out = outer.to_vec();
    close_poly_if_needed(&mut out);

    let mut hole_wires = Vec::with_capacity(holes.len());
    for h in holes {
        let mut hh = h.clone();
        close_poly_if_needed(&mut hh);
        hole_wires.push(wire_from_closed_polyline(&hh));
    }

    // 2) Wire → Face → Solid
    let outer_wire = wire_from_closed_polyline(&out);
    let face = planar_face_from_wires(&outer_wire, &hole_wires);
    let solid = extrude_face(&face, height);

    // 3) Solid → TriMesh
    let (mut positions, indices) = mesh_from_solid(&solid);

    // 4) Применяем матрицу (если рендер не умножает на GPU)
    transform_positions_inplace(&mut positions, xform_row_major);

    (positions, indices)
}

/// Упрощённый вариант без отверстий; матрицу тоже применяем.
pub fn extrude_polyline_to_mesh_with_transform(
    poly_closed: &[Pt2],
    height: f64,
    xform_row_major: &[[f32; 4]; 4],
) -> (Vec<[f32; 3]>, Vec<u32>) {
    extrude_polygon_to_mesh_with_transform(poly_closed, &[], height, xform_row_major)
}

/// (Опционально) «псевдо-точная» проволока из произвольной EntityKind.
/// Сейчас — через дискретизацию в полилинию (чтобы сразу работало для дуг/НУРБС).
/// Если захочешь делать точные ребра NURBS — можно дописать отдельную ветку.
pub fn wire_from_entity_kind_sampled(kind: &EntityKind, chord_tol: f64) -> Option<Wire> {
    match kind {
        EntityKind::Polyline { pts, closed } => {
            let mut poly = pts.clone();
            if *closed {
                close_poly_if_needed(&mut poly);
            }
            if poly.len() >= 3 {
                Some(wire_from_closed_polyline(&poly))
            } else {
                None
            }
        }
        EntityKind::LineSeg { a, b } => {
            let mut poly = vec![*a, *b, *a];
            Some(wire_from_closed_polyline(&poly))
        }
        EntityKind::Arc { center, radius, start_angle, end_angle } => {
            // Дискретизируем дугу в полилинию
            let segs = ((end_angle - start_angle).abs() / std::f32::consts::PI * 24.0)
                .max(8.0)
                .round() as usize;
            let c = *center;
            let r = *radius as f64;
            let a0 = *start_angle as f64;
            let a1 = *end_angle as f64;
            let mut poly = Vec::with_capacity(segs + 1);
            for i in 0..=segs {
                let t = i as f64 / (segs as f64);
                let a = a0 * (1.0 - t) + a1 * t;
                poly.push(Pt2::new((c.x as f64 + r * a.cos()) as f32,
                                   (c.y as f64 + r * a.sin()) as f32));
            }
            close_poly_if_needed(&mut poly);
            Some(wire_from_closed_polyline(&poly))
        }
        EntityKind::NurbsCurve2D { .. } => {
            // Дискретизация через общую выборку из твоего слоя (если она у тебя уже есть).
            // Здесь сделаем простую подстановку: попробуем равномерно по параметру.
            // TODO: заменить на твою реализацию sample(kind, chord_tol).
            let mut poly = sample_entity_kind(kind, chord_tol);
            close_poly_if_needed(&mut poly);
            if poly.len() >= 3 {
                Some(wire_from_closed_polyline(&poly))
            } else {
                None
            }
        }
        EntityKind::Text { .. } => None,
    }
}

/// Примитивная дискретизация EntityKind → полилиния (для НУРБС/дуг).
/// Вынесено отдельно, чтобы можно было заменить на твою «умную» выборку по допуску.
fn sample_entity_kind(kind: &EntityKind, _chord_tol: f64) -> Vec<Pt2> {
    match kind {
        EntityKind::Polyline { pts, .. } => pts.clone(),
        EntityKind::LineSeg { a, b } => vec![*a, *b],
        EntityKind::Arc {
            center,
            radius,
            start_angle,
            end_angle,
        } => {
            let segs = ((end_angle - start_angle).abs() / std::f32::consts::PI * 32.0)
                .max(12.0)
                .round() as usize;
            let c = *center;
            let r = *radius as f64;
            let a0 = *start_angle as f64;
            let a1 = *end_angle as f64;
            let mut poly = Vec::with_capacity(segs + 1);
            for i in 0..=segs {
                let t = i as f64 / (segs as f64);
                let a = a0 * (1.0 - t) + a1 * t;
                poly.push(Pt2::new((c.x as f64 + r * a.cos()) as f32,
                                   (c.y as f64 + r * a.sin()) as f32));
            }
            poly
        }
        EntityKind::NurbsCurve2D {
            degree,
            knots,
            ctrl_pts,
            weights,
        } => {
            // Быстрый и универсальный способ: построим BSplineCurve на плоскости (без весов для простоты)
            // и снимем точки равномерно по параметру.
            let ctrl: Vec<Point2<f64>> = ctrl_pts.iter().map(|q| p2(*q)).collect();
            let kv = KnotVec::from(knots.clone());
            let bs: BSplineCurve<Point2<f64>> = BSplineCurve::new(kv, ctrl);
            let n = 64usize.max(4 * (degree + 1));
            (0..=n)
                .map(|i| {
                    let t = i as f64 / n as f64;
                    let pt = bs.subs(t);
                    Pt2::new(pt.x as f32, pt.y as f32)
                })
                .collect()
        }
        EntityKind::Text { .. } => vec![],
    }
}
