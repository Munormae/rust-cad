//! Meshing algorithms for cryxtal geometry/topology: tessellation, filters, analyzers.
//!
//! This crate provides polygonization, normal generation, topology analyzers and VTK export
//! over `cryxtal-polymesh` primitives. Use `prelude` to import the commonly used items.
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

use common::*;

/// Re-export of the underlying polymesh primitives.
pub mod rexport_polymesh {
    pub use cryxtal_polymesh::*;
}
use cryxtal_polymesh::{StandardVertex as Vertex, *};

/// Geometry/topology analyzers (intersection, in/out tests, etc.).
#[cfg(feature = "analyzers")]
pub mod analyzers;
mod common;
/// Mesh filters: smoothing, optimizing, structuring, subdivision.
#[cfg(feature = "filters")]
pub mod filters;
/// Tessellation routines for curves, surfaces and shapes.
#[cfg(feature = "tessellation")]
pub mod tessellation;

/// Optional VTK I/O utilities (non-wasm32 targets).
#[cfg(feature = "vtk")]
#[cfg(not(target_arch = "wasm32"))]
pub mod vtk;

/// Convenience prelude exporting feature-gated modules and polymesh basics.
pub mod prelude {
    #[cfg(feature = "analyzers")]
    pub use crate::analyzers::*;
    #[cfg(feature = "filters")]
    pub use crate::filters::*;
    pub use crate::rexport_polymesh::*;
    #[cfg(feature = "tessellation")]
    pub use crate::tessellation::*;
    #[cfg(feature = "vtk")]
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::vtk::*;
}
