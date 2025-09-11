// cad-core/src/ifc_io.rs
//! IFC importer (Lite): читаем STEP p21 построчно до ';', парсим #id=TYPE(args)
//! и конвертим в наш Project3D/Model3D/Element3D (Extrusion + SweptDisk).

use anyhow::{anyhow, Result};
use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

use crate::model3d::*;
use crate::Pt2;

// ============================== PUBLIC API ===============================

pub fn import_ifc(path: &str) -> Result<Project3D> {
    import_ifc_lite(path)
}

// =========================== LITE BACKEND ================================

fn import_ifc_lite(path: &str) -> Result<Project3D> {
    let text = std::fs::read_to_string(path).map_err(|e| anyhow!("read IFC failed: {e}"))?;

    // 1) сканируем p21
    let stmts = scan_p21(&text);
    let mut db = IfcDb::default();
    db.ingest(stmts);

    let scale_to_mm = db.units_scale_to_mm(); // f32

    // 2) строим модель
    let mut model = Model3D {
        name: std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("ifc")
            .to_string(),
        ..Default::default()
    };

    let mut n_ex = 0usize;
    let mut n_sw = 0usize;
    let n_rb = 0usize;

    // === ExtrudedAreaSolid ===
    for (id, ex) in db.extruded.iter() {
        if let Some((profile_pts, height)) = db.extrusion_profile(*ex) {
            let xform = db.placement_to_xform(*&ex.axis, scale_to_mm);
            let s = scale_to_mm as f64;
            let poly: Vec<Pt2> = profile_pts
                .into_iter()
                .map(|p| Pt2::new((p.x * s) as f32, (p.y * s) as f32))
                .collect();

            model.elements.push(Element3D {
                id: *id as u64,
                name: format!("E{}", id),
                xform,
                geom: ElementGeom::Extrusion {
                    profile: poly,
                    height: (height * s) as f32,
                },
                material: 0,
                rebars: vec![],
                meta: Meta::default(),
            });
            n_ex += 1;
        }
    }

    // === SweptDiskSolid ===
    for (id, sd) in db.swept.iter() {
        if let Some((path3, radius)) = db.swept_path(*sd) {
            let s = scale_to_mm as f64;
            let xform = [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ];
            model.elements.push(Element3D {
                id: *id as u64,
                name: format!("S{}", id),
                xform,
                geom: ElementGeom::SweepCylinder {
                    path: path3
                        .into_iter()
                        .map(|p| Pt3 {
                            x: (p.x * s) as f32,
                            y: (p.y * s) as f32,
                            z: (p.z * s) as f32,
                        })
                        .collect(),
                    radius: (radius * s) as f32,
                },
                material: 0,
                rebars: vec![],
                meta: Meta::default(),
            });
            n_sw += 1;
        }
    }

    eprintln!(
        "[IFC] Imported: extrusions={n_ex}, sweeps={n_sw}, rebars={n_rb} → total elems={}",
        model.elements.len()
    );

    Ok(Project3D {
        models: vec![model],
    })
}

// ===================== Сканер p21 (простенький) ==========================

#[derive(Debug, Clone)]
struct Stmt {
    id: u32,
    kind: String,
    args: String,
}

fn scan_p21(text: &str) -> Vec<Stmt> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*)\)\s*;$").unwrap();
    }
    let mut out = Vec::new();
    let mut buf = String::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with("//") {
            continue;
        }
        buf.push_str(l);
        if l.ends_with(';') {
            if let Some(cap) = RE.captures(&buf) {
                let id: u32 = cap
                    .get(1)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0);
                let kind = cap
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let args = cap
                    .get(3)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                out.push(Stmt { id, kind, args });
            }
            buf.clear();
        } else {
            buf.push(' ');
        }
    }
    out
}

// ========================= Мини-БД IFC ===================================

#[derive(Debug, Clone, Copy, Default)]
struct P3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Default)]
struct IfcDb {
    // базовые:
    pts: HashMap<u32, P3>,                             // IFCCARTESIANPOINT
    rects: HashMap<u32, (f64, f64)>,                   // IFCRECTANGLEPROFILEDEF -> (x,y)
    polylines: HashMap<u32, Vec<u32>>,                 // IFCPOLYLINE -> [#pt]
    arb_prof: HashMap<u32, u32>,                       // IFCARBITRARYCLOSEDPROFILEDEF -> #polyline
    axis3: HashMap<u32, u32>,                          // IFCAXIS2PLACEMENT3D -> #location (point)
    local_placement: HashMap<u32, (Option<u32>, u32)>, // IFClocal (rel, placement)

    // тела:
    extruded: HashMap<u32, ExtrudedRec>, // id -> data
    swept: HashMap<u32, SweptRec>,       // id -> data

    // units
    units: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct ExtrudedRec {
    profile: u32,      // #profile
    axis: Option<u32>, // #IFCAXIS2PLACEMENT3D
    depth: f64,
}

#[derive(Debug, Clone, Copy)]
struct SweptRec {
    directrix: u32, // #IFCPOLYLINE (MVP)
    radius: f64,
}

impl Default for ExtrudedRec {
    fn default() -> Self {
        Self {
            profile: 0,
            axis: None,
            depth: 0.0,
        }
    }
}
impl Default for SweptRec {
    fn default() -> Self {
        Self {
            directrix: 0,
            radius: 0.0,
        }
    }
}

impl IfcDb {
    fn ingest(&mut self, stmts: Vec<Stmt>) {
        lazy_static! {
            static ref RE_NUMS: Regex = Regex::new(r"[-+]?\d*\.?\d+(?:[Ee][-+]?\d+)?").unwrap();
            static ref RE_IDREF: Regex = Regex::new(r"#(\d+)").unwrap();
        }

        for s in stmts {
            match s.kind.as_str() {
                "IFCSIUNIT" | "IFCUNITASSIGNMENT" => {
                    self.units.push(s.args);
                }
                "IFCCARTESIANPOINT" => {
                    // ((x,y,z))
                    let nums: Vec<f64> = RE_NUMS
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().parse().ok())
                        .collect();
                    let p = P3 {
                        x: *nums.get(0).unwrap_or(&0.0),
                        y: *nums.get(1).unwrap_or(&0.0),
                        z: *nums.get(2).unwrap_or(&0.0),
                    };
                    self.pts.insert(s.id, p);
                }
                "IFCPOLYLINE" => {
                    // ((#1,#2,#3))
                    let ids: Vec<u32> = RE_IDREF
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().trim_start_matches('#').parse().ok())
                        .collect();
                    self.polylines.insert(s.id, ids);
                }
                "IFCRECTANGLEPROFILEDEF" => {
                    // ... XDim, YDim ...
                    let nums: Vec<f64> = RE_NUMS
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().parse().ok())
                        .collect();
                    let x = *nums.get(0).unwrap_or(&100.0);
                    let y = *nums.get(1).unwrap_or(&100.0);
                    self.rects.insert(s.id, (x, y));
                }
                "IFCARBITRARYCLOSEDPROFILEDEF" => {
                    // ... (#polyline, ...)
                    if let Some(cap) = RE_IDREF.captures(&s.args) {
                        if let Some(m1) = cap.get(1) {
                            if let Ok(pid) = m1.as_str().parse() {
                                self.arb_prof.insert(s.id, pid);
                            }
                        }
                    }
                }
                "IFCAXIS2PLACEMENT3D" => {
                    // (#location, #axis?, #refdir?)
                    if let Some(cap) = RE_IDREF.captures(&s.args) {
                        if let Some(m1) = cap.get(1) {
                            if let Ok(pid) = m1.as_str().parse() {
                                self.axis3.insert(s.id, pid);
                            }
                        }
                    }
                }
                "IFCLOCALPLACEMENT" => {
                    // ($ | #rel, #axis2placement3d)
                    let ids: Vec<u32> = RE_IDREF
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().trim_start_matches('#').parse().ok())
                        .collect();
                    let rel = ids.get(0).copied();
                    let place = *ids.get(1).unwrap_or(&0);
                    self.local_placement.insert(s.id, (rel, place));
                }
                "IFCEXTRUDEDAREASOLID" => {
                    // (#profile, #axis?, #direction, depth) — вариативно.
                    let ids: Vec<u32> = RE_IDREF
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().trim_start_matches('#').parse().ok())
                        .collect();
                    let nums: Vec<f64> = RE_NUMS
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().parse().ok())
                        .collect();
                    let profile = *ids.get(0).unwrap_or(&0);
                    let axis = ids.get(1).copied(); // эвристика
                    let depth = *nums.last().unwrap_or(&100.0);
                    self.extruded.insert(
                        s.id,
                        ExtrudedRec {
                            profile,
                            axis,
                            depth,
                        },
                    );
                }
                "IFCSWEPTDISKSOLID" => {
                    // (#directrix, radius, ...)
                    let ids: Vec<u32> = RE_IDREF
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().trim_start_matches('#').parse().ok())
                        .collect();
                    let nums: Vec<f64> = RE_NUMS
                        .find_iter(&s.args)
                        .filter_map(|m| m.as_str().parse().ok())
                        .collect();
                    let directrix = *ids.get(0).unwrap_or(&0);
                    let radius = *nums.get(0).unwrap_or(&8.0);
                    self.swept.insert(s.id, SweptRec { directrix, radius });
                }
                _ => {}
            }
        }
    }

    fn units_scale_to_mm(&self) -> f32 {
        let joined = self.units.join(" ");
        if joined.contains(".MILLI.") {
            1.0
        } else if joined.contains(".METRE.") {
            1000.0
        } else {
            1.0
        }
    }

    fn placement_to_xform(&self, axis: Option<u32>, s: f32) -> [[f32; 4]; 4] {
        if let Some(aid) = axis {
            if let Some(loc_pt_id) = self.axis3.get(&aid) {
                if let Some(p) = self.pts.get(loc_pt_id) {
                    return [
                        [1.0, 0.0, 0.0, (p.x as f32) * s],
                        [0.0, 1.0, 0.0, (p.y as f32) * s],
                        [0.0, 0.0, 1.0, (p.z as f32) * s],
                        [0.0, 0.0, 0.0, 1.0],
                    ];
                }
            }
        }
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn extrusion_profile(&self, ex: ExtrudedRec) -> Option<(Vec<P3>, f64)> {
        if let Some((x, y)) = self.rects.get(&ex.profile) {
            let pts = vec![
                P3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                P3 {
                    x: *x,
                    y: 0.0,
                    z: 0.0,
                },
                P3 {
                    x: *x,
                    y: *y,
                    z: 0.0,
                },
                P3 {
                    x: 0.0,
                    y: *y,
                    z: 0.0,
                },
                P3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ];
            return Some((pts, ex.depth));
        }
        if let Some(poly_id) = self.arb_prof.get(&ex.profile) {
            if let Some(poly) = self.polyline_pts(*poly_id) {
                return Some((poly, ex.depth));
            }
        }
        None
    }

    fn swept_path(&self, sd: SweptRec) -> Option<(Vec<P3>, f64)> {
        let path = self.polyline_pts(sd.directrix)?;
        Some((path, sd.radius))
    }

    fn polyline_pts(&self, poly_id: u32) -> Option<Vec<P3>> {
        let ids = self.polylines.get(&poly_id)?;
        let mut out = Vec::with_capacity(ids.len());
        for pid in ids {
            if let Some(p) = self.pts.get(pid) {
                out.push(*p);
            }
        }
        Some(out)
    }
}
