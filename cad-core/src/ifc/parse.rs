// cad-core/src/ifc/parse.rs
use super::ast::*;
use super::scanner::read_p21_entities;
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref RE_ID:    Regex = Regex::new(r"^#(\d+)\s*=\s*([A-Z0-9_]+)\((.*)\);$").unwrap();
    static ref RE_NUMS:  Regex = Regex::new(r"[-+]?\d*\.?\d+(?:[Ee][-+]?\d+)?").unwrap();
    static ref RE_IDREF: Regex = Regex::new(r"#(\d+)").unwrap();
}

// ---------- helpers ----------
#[inline]
fn all_nums(s: &str) -> Vec<f64> {
    RE_NUMS
        .find_iter(s)
        .filter_map(|m| m.as_str().parse::<f64>().ok())
        .collect()
}

#[inline]
fn all_ids(s: &str) -> Vec<Idx> {
    // используем captures_iter, чтобы забирать именно группу (цифры), а не всю подстроку "#123"
    RE_IDREF
        .captures_iter(s)
        .filter_map(|c| c.get(1).and_then(|m| m.as_str().parse::<u32>().ok()))
        .map(Idx)
        .collect()
}

#[inline]
fn first_id(s: &str) -> Option<Idx> {
    RE_IDREF
        .captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
        .map(Idx)
}

// ---------- main ----------
pub fn parse_db(text: &str) -> Result<Db> {
    let mut db = Db::default();
    for stmt in read_p21_entities(text) {
        if let Some(caps) = RE_ID.captures(&stmt) {
            let id: u32 = caps[1].parse().unwrap_or(0);
            let kind: &str = &caps[2];
            let args: &str = caps[3].trim();

            let ent = match kind {
                "IFCSIUNIT" | "IFCUNITASSIGNMENT" => parse_units(args),
                "IFCCARTESIANPOINT"               => parse_cartesian_point(args),
                "IFCDIRECTION"                    => parse_direction(args),
                "IFCPOLYLINE"                     => parse_polyline(args),
                "IFCRECTANGLEPROFILEDEF"          => parse_rectangle_profile(args),
                "IFCARBITRARYCLOSEDPROFILEDEF"    => parse_arbitrary_profile(args),
                "IFCAXIS2PLACEMENT3D"             => parse_axis2placement3d(args),
                "IFCLOCALPLACEMENT"               => parse_local_placement(args),
                "IFCEXTRUDEDAREASOLID"            => parse_extruded(args),
                "IFCSWEPTDISKSOLID"               => parse_swept_disk(args),
                "IFCPRODUCT" | "IFCBUILDINGELEMENT" | "IFCCOLUMN" | "IFCBEAM" | "IFCPLATE" |
                "IFCSLAB" | "IFCWALL" | "IFCMEMBER" | "IFCREINFORCINGBAR" | "IFCREINFORCINGELEMENT" => {
                    parse_product(args).unwrap_or(Entity::Unknown)
                }
                "IFCSHAPEREPRESENTATION" | "IFCSHAPEMODEL" => parse_shape_rep(args),
                _ => Entity::Unknown,
            };

            db.insert(id, ent);
        }
    }
    Ok(db)
}

fn parse_units(args: &str) -> Entity {
    if args.contains(".LENGTHUNIT.") {
        if args.contains(".MILLI.") { Entity::UnitAssignment(UnitLen::MilliMetre) }
        else if args.contains(".METRE.") { Entity::UnitAssignment(UnitLen::Metre) }
        else { Entity::UnitAssignment(UnitLen::Metre) }
    } else {
        Entity::Unknown
    }
}

fn parse_cartesian_point(args: &str) -> Entity {
    let nums = all_nums(args);
    let x = *nums.get(0).unwrap_or(&0.0);
    let y = *nums.get(1).unwrap_or(&0.0);
    let z = *nums.get(2).unwrap_or(&0.0);
    Entity::CartesianPoint(Point3 { x, y, z })
}

fn parse_direction(args: &str) -> Entity {
    let nums = all_nums(args);
    let x = *nums.get(0).unwrap_or(&1.0);
    let y = *nums.get(1).unwrap_or(&0.0);
    let z = *nums.get(2).unwrap_or(&0.0);
    Entity::Direction(Point3 { x, y, z })
}

fn parse_polyline(args: &str) -> Entity {
    Entity::Polyline(all_ids(args))
}

fn parse_rectangle_profile(args: &str) -> Entity {
    // XDim, YDim — первые две числовые
    let nums = all_nums(args);
    let x = *nums.get(0).unwrap_or(&100.0);
    let y = *nums.get(1).unwrap_or(&100.0);
    Entity::RectangleProfile { x, y }
}

fn parse_arbitrary_profile(args: &str) -> Entity {
    let poly = first_id(args).unwrap_or(Idx(0));
    Entity::ArbitraryClosedProfile { poly }
}

fn parse_axis2placement3d(args: &str) -> Entity {
    // ( #location, #axis(z)? , #refDirection(x)? )
    let ids = all_ids(args);
    let location = ids.get(0).copied().unwrap_or(Idx(0));
    let dir_z    = ids.get(1).copied();
    let dir_x    = ids.get(2).copied();
    Entity::Axis2Placement3D { location, dir_z, dir_x }
}

fn parse_local_placement(args: &str) -> Entity {
    // ( $ | #rel, #axis2placement3d )
    let ids = all_ids(args);
    let rel       = ids.get(0).copied();
    let placement = ids.get(1).copied().unwrap_or(Idx(0));
    Entity::LocalPlacement { rel, placement }
}

fn parse_extruded(args: &str) -> Entity {
    // ( #profile, #axis?, #direction, Depth ) — сигнатуры разнятся
    let ids  = all_ids(args);
    let nums = all_nums(args);

    let profile = *ids.get(0).unwrap_or(&Idx(0));
    let axis    = ids.get(1).copied();
    let dir     = ids.get(2).copied();
    let depth   = *nums.last().unwrap_or(&100.0);

    Entity::ExtrudedAreaSolid { profile, axis, depth, dir }
}

fn parse_swept_disk(args: &str) -> Entity {
    // ( #directrix, radius, [inner]? )
    let ids  = all_ids(args);
    let nums = all_nums(args);

    let directrix = *ids.get(0).unwrap_or(&Idx(0));
    let radius    = *nums.get(0).unwrap_or(&8.0);
    Entity::SweptDiskSolid { directrix, radius }
}

fn parse_product(args: &str) -> Option<Entity> {
    // (..., ObjectPlacement, Representation, ... )
    let ids = all_ids(args);
    let op  = ids.get(0).copied();
    let rep = ids.get(1).copied();
    Some(Entity::Product { object_placement: op, rep, name: None })
}

fn parse_shape_rep(args: &str) -> Entity {
    Entity::ShapeRep { items: all_ids(args) }
}
