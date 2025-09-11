// cad-core/src/ifc/ast.rs
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Idx(pub u32); // индекс вида #123

#[derive(Debug, Clone)]
pub enum Entity {
    UnitAssignment(UnitLen),
    CartesianPoint(Point3),
    Polyline(Vec<Idx>), // ссылки на CartesianPoint
    RectangleProfile { x: f64, y: f64 },
    ArbitraryClosedProfile { poly: Idx }, // IfcPolyline
    Axis2Placement3D { location: Idx, dir_z: Option<Idx>, dir_x: Option<Idx> }, // dirs — IfcDirection
    Direction(Point3), // используем как вектор
    LocalPlacement { rel: Option<Idx>, placement: Idx }, // Axis2Placement3D
    ExtrudedAreaSolid {
        profile: Idx,         // RectangleProfile / ArbitraryClosedProfile
        axis: Option<Idx>,    // Axis2Placement3D (локальная ось)
        depth: f64,           // мм после масштабирования
        dir: Option<Idx>,     // IfcDirection
    },
    SweptDiskSolid {
        directrix: Idx,       // IfcPolyline
        radius: f64,
    },
    Product { object_placement: Option<Idx>, rep: Option<Idx>, name: Option<String> },
    ShapeRep { items: Vec<Idx> }, // ссылки на геом. объекты (Extruded..., SweptDisk..., Brep etc.)
    Unknown, // всё остальное игнорим
}

#[derive(Debug, Clone, Copy)]
pub struct Point3 { pub x: f64, pub y: f64, pub z: f64 }

#[derive(Debug, Clone, Copy)]
pub enum UnitLen {
    MilliMetre,
    Metre,
}

#[derive(Default)]
pub struct Db {
    pub map: HashMap<u32, Entity>,
    pub names: HashMap<u32, String>, // #id → имя (если удаётся вытащить)
}

impl Db {
    pub fn insert(&mut self, id: u32, e: Entity) { self.map.insert(id, e); }
    pub fn get(&self, id: Idx) -> Option<&Entity> { self.map.get(&id.0) }
    pub fn scale_factor_mm(&self) -> f64 {
        for (_id, e) in &self.map {
            if let Entity::UnitAssignment(u) = e {
                return match u { UnitLen::MilliMetre => 1.0, UnitLen::Metre => 1000.0 };
            }
        }
        1.0
    }
}
