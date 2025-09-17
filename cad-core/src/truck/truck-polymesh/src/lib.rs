#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

use array_macro::array;
use serde::{Deserialize, Serialize};

pub mod base {
    pub use truck_base::{
        assert_near, assert_near2, bounding_box::BoundingBox, cgmath64::*, hash, hash::HashGen,
        prop_assert_near, prop_assert_near2, tolerance::*,
    };
    pub use truck_geotrait::*;
}
pub use base::*;

pub trait Attributes<V> {
    type Output;
    fn get(&self, vertex: V) -> Option<Self::Output>;
}

pub trait TransformedAttributes: Clone {
    fn transform_by(&mut self, trans: Matrix4);
    fn transformed(&self, trans: Matrix4) -> Self;
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StandardAttributes {
    pub positions: Vec<Point3>,
    pub uv_coords: Vec<Vector2>,
    pub normals: Vec<Vector3>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StandardAttribute {
    pub position: Point3,
    pub uv_coord: Option<Vector2>,
    pub normal: Option<Vector3>,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StandardVertex {
    pub pos: usize,
    pub uv: Option<usize>,
    pub nor: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faces<V = StandardVertex> {
    tri_faces: Vec<[V; 3]>,
    quad_faces: Vec<[V; 4]>,
    other_faces: Vec<Vec<V>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PolygonMesh<V = StandardVertex, A = StandardAttributes> {
    attributes: A,
    faces: Faces<V>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StructuredMesh {
    positions: Vec<Vec<Point3>>,
    uv_division: Option<(Vec<f64>, Vec<f64>)>,
    normals: Option<Vec<Vec<Vector3>>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolylineCurve<P>(pub Vec<P>);

mod attributes;
pub mod errors;
mod expand;
pub mod faces;
mod meshing_shape;
pub mod obj;
pub mod polygon_mesh;
pub mod polyline_curve;
pub mod stl;
mod structured_mesh;
