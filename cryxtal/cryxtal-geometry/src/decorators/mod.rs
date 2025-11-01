// cryxtal-geometry/src/decorators/mod.rs
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Mul};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Revolution {
    origin: Point3,
    axis: Vector3,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct RevolutedCurve<C> {
    curve: C,
    revolution: Revolution,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct ExtrudedCurve<C, V> {
    curve: C,
    vector: V,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Processor<E, T> {
    entity: E,
    transform: T,
    orientation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct PCurve<C, S> {
    curve: C,
    surface: S,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct IntersectionCurve<C, S0, S1> {
    surface0: S0,
    surface1: S1,
    leader: C,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct TrimmedCurve<C> {
    curve: C,
    range: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct HomotopySurface<C0, C1> {
    curve0: C0,
    curve1: C1,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct RbfSurface<C, S0, S1, R> {
    edge_curve: C,
    surface0: S0,
    surface1: S1,
    radius: R,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct RbfContactCurve<C, S0, S1, R> {
    surface: RbfSurface<C, S0, S1, R>,
    index: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct ApproxFilletSurface<S0, S1> {
    knot_vec: KnotVec,
    surface0: S0,
    side_control_points0: Vec<Point2>,
    tangent_vecs0: Vec<Vector2>,
    surface1: S1,
    side_control_points1: Vec<Point2>,
    tangent_vecs1: Vec<Vector2>,
    weights: Vec<f64>,
}

// ── подмодули
mod af_surface;
mod extruded_curve;
mod homotopy;
mod intersection_curve;
mod pcurve;
mod processor;
pub mod rbf_surface;
mod revolved_curve;
mod trimmed_curve; // если нужен прямой доступ из вне

// ── реэкспортируем публичное API (никаких `use ...`, только `pub use`)
pub use af_surface::*;
pub use extruded_curve::*;
pub use homotopy::*;
pub use intersection_curve::*;
pub use pcurve::*;
pub use processor::*;
pub use rbf_surface::*;
pub use revolved_curve::*;
pub use trimmed_curve::*;
