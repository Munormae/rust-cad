// cad-core/src/ifc/import.rs
use anyhow::{anyhow, Result};
use std::vec;
use std::format;
use crate::Pt2;

// 3D-модель вашего ядра.
// Если у вас другие имена — поправьте пути/типы ниже.
use crate::model3d::{
    Element3D, ElementGeom, Meta, Model3D, Project3D, Pt3,
};

use crate::ifc::ast::{Db, Entity, Idx, Point3};
use crate::ifc::parse::parse_db;
use crate::ifc::scanner::load_file;

/// Загрузить IFC и собрать грубую 3D-модель:
/// - ExtrudedAreaSolid → призма по профилю (Rectangle/ArbitraryClosed) с высотой;
/// - SweptDiskSolid   → «труба/стержень» по 3D-полилинии.
pub fn import_ifc(path: &str) -> Result<Project3D> {
    let text = load_file(path)?;
    let db: Db = parse_db(&text)?;
    let s = db.scale_factor_mm(); // перевод в мм, если в файле метры

    let mut model = Model3D {
        name: file_stem(path),
        ..Default::default()
    };

    // MVP: читаем только твердотельную геометрию, без учёта иерархий продуктов
    for (id, ent) in db.map.iter() {
        match ent {
            Entity::ExtrudedAreaSolid { profile, axis, depth, .. } => {
                if let Some(geom) = build_extrusion(&db, *profile, *depth, s)? {
                    let xform = axis_to_matrix(&db, *axis, s);
                    model.elements.push(Element3D {
                        id: *id as u64,
                        name: format!("EXTRUDED_{}", id),
                        xform,
                        geom,
                        material: 0,
                        rebars: vec![],
                        meta: Meta::default(),
                    });
                }
            }
            Entity::SweptDiskSolid { directrix, radius } => {
                if let Some(geom) = build_swept(&db, *directrix, *radius, s)? {
                    // пока без трансформаций
                    let xform = ID4;
                    model.elements.push(Element3D {
                        id: *id as u64,
                        name: format!("SWEPT_{}", id),
                        xform,
                        geom,
                        material: 0,
                        rebars: vec![],
                        meta: Meta::default(),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(Project3D { models: vec![model] })
}

const ID4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn file_stem(p: &str) -> String {
    std::path::Path::new(p)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ifc")
        .to_string()
}

/// Перевод Axis2Placement3D в 4×4 матрицу (пока только перенос из location).
fn axis_to_matrix(db: &Db, axis: Option<Idx>, s: f64) -> [[f32; 4]; 4] {
    if let Some(Idx(aid)) = axis {
        if let Some(Entity::Axis2Placement3D { location, .. }) = db.map.get(&aid) {
            if let Some(Entity::CartesianPoint(p)) = db.get(*location) {
                return [
                    [1.0, 0.0, 0.0, (p.x * s) as f32],
                    [0.0, 1.0, 0.0, (p.y * s) as f32],
                    [0.0, 0.0, 1.0, (p.z * s) as f32],
                    [0.0, 0.0, 0.0, 1.0],
                ];
            }
        }
    }
    ID4
}

/// Построить геометрию выдавливания.
fn build_extrusion(db: &Db, profile: Idx, depth: f64, s: f64) -> Result<Option<ElementGeom>> {
    let height = (depth * s) as f32;
    let geom = match db.get(profile) {
        Some(Entity::RectangleProfile { x, y }) => {
            let (x, y) = ((*x * s) as f32, (*y * s) as f32);
            // прямоугольник от (0,0)
            let poly = vec![
                Pt2::new(0.0, 0.0),
                Pt2::new(x, 0.0),
                Pt2::new(x, y),
                Pt2::new(0.0, y),
                Pt2::new(0.0, 0.0),
            ];
            ElementGeom::Extrusion { profile: poly, height }
        }
        Some(Entity::ArbitraryClosedProfile { poly }) => {
            let pts = polyline_points_mm(db, *poly, s)?;
            ElementGeom::Extrusion { profile: pts, height }
        }
        _ => return Ok(None),
    };
    Ok(Some(geom))
}

/// Построить SweptDiskSolid (цилиндрическая «труба/стержень» вдоль 3D-полилинии).
fn build_swept(db: &Db, directrix: Idx, radius: f64, s: f64) -> Result<Option<ElementGeom>> {
    let path3 = polyline_points3_mm(db, directrix, s)?;
    let path: Vec<Pt3> = path3
        .into_iter()
        .map(|p| Pt3 {
            x: p.x as f32,
            y: p.y as f32,
            z: p.z as f32,
        })
        .collect();
    Ok(Some(ElementGeom::SweepCylinder {
        path,
        radius: (radius * s) as f32,
    }))
}

/// 2D-полилиния (XY) в мм — для профилей.
fn polyline_points_mm(db: &Db, poly: Idx, s: f64) -> Result<Vec<Pt2>> {
    if let Some(Entity::Polyline(ids)) = db.get(poly) {
        let mut out = Vec::with_capacity(ids.len());
        for &Idx(pid) in ids {
            if let Some(Entity::CartesianPoint(p)) = db.map.get(&pid) {
                out.push(Pt2::new((p.x * s) as f32, (p.y * s) as f32));
            }
        }
        return Ok(out);
    }
    Err(anyhow!("Polyline not found"))
}

/// 3D-полилиния (XYZ) в мм — для траекторий.
fn polyline_points3_mm(db: &Db, poly: Idx, s: f64) -> Result<Vec<Point3>> {
    if let Some(Entity::Polyline(ids)) = db.get(poly) {
        let mut out = Vec::with_capacity(ids.len());
        for &Idx(pid) in ids {
            if let Some(Entity::CartesianPoint(p)) = db.map.get(&pid) {
                out.push(Point3 {
                    x: p.x * s,
                    y: p.y * s,
                    z: p.z * s,
                });
            }
        }
        return Ok(out);
    }
    Err(anyhow!("Polyline not found"))
}
