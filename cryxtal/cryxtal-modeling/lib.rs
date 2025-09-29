//! Cryxtal modeling layer: higher-level modeling primitives and builders over geometry/topology.
//!
//! This crate defines user-facing types (`Curve`, `Surface`) and builders wrapping
//! lower-level cryxtal geometry/topology. It re-exports commonly used base types via
//! the `base` module and provides helper traits in `topo_traits`.
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

/// Public re-exports of base math, tolerance and geotrait symbols used across modeling.
pub mod base {
    pub use cryxtal_base::{
        assert_near, assert_near2, bounding_box::BoundingBox, cgmath64::*, prop_assert_near,
        prop_assert_near2, tolerance::*,
    };
    pub use cryxtal_geotrait::*;
}
/// Modeling geometry enums and implementations over cryxtal-geometry.
pub mod geometry;
pub use geometry::*;
/// Type aliases and prelude for cryxtal-topology specialized by modeling `Point3`, `Curve`, `Surface`.
pub mod topology {
    use crate::{base::Point3, Curve, Surface};
    cryxtal_topology::prelude!(Point3, Curve, Surface, pub);
}
/// Small helper traits for mapping and sweeping utilities used by builders.
pub mod topo_traits {
    pub trait GeometricMapping<T>: Copy {
        fn mapping(self) -> impl Fn(&T) -> T;
    }
    pub trait Connector<T, H>: Copy {
        fn connector(self) -> impl Fn(&T, &T) -> H;
    }
    pub trait Mapped<T>: Sized {
        #[doc(hidden)]
        fn mapped(&self, trans: T) -> Self;
    }

    pub trait Sweep<T, Pc, Cc, Swept> {
        fn sweep(&self, trans: T, point_connector: Pc, curve_connector: Cc) -> Swept;
    }

    pub trait MultiSweep<T, Pc, Cc, Swept> {
        fn multi_sweep(
            &self,
            trans: T,
            point_connector: Pc,
            curve_connector: Cc,
            division: usize,
        ) -> Swept;
    }

    pub trait ClosedSweep<T, Pc, Cc, Swept>: MultiSweep<T, Pc, Cc, Swept> {
        fn closed_sweep(
            &self,
            trans: T,
            point_connector: Pc,
            curve_connector: Cc,
            division: usize,
        ) -> Swept;
    }
}
pub use topo_traits::*;
/// Convenient result alias for this crate.
pub type Result<T> = std::result::Result<T, errors::Error>;
/// High-level constructors for topology (vertices, edges, faces, shells, solids).
pub mod builder;
mod closed_sweep;
/// Modeling error types.
pub mod errors;
mod geom_impls;
mod mapped;
mod multi_sweep;
/// Ready-to-use primitive constructors (rect, circle, cuboid, etc.).
pub mod primitive;
mod sweep;
mod topo_impls;
