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

pub mod base {
    pub use truck_base::{
        assert_near, assert_near2, bounding_box::BoundingBox, cgmath64::*, prop_assert_near,
        prop_assert_near2, tolerance::*,
    };
    pub use truck_geotrait::*;
}
pub mod geometry;
pub use geometry::*;
pub mod topology {
    use crate::{Curve, Point3, Surface};
    truck_topology::prelude!(Point3, Curve, Surface, pub);
}
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
pub type Result<T> = std::result::Result<T, errors::Error>;
pub mod builder;
mod closed_sweep;
pub mod errors;
mod geom_impls;
mod mapped;
mod multi_sweep;
pub mod primitive;
mod sweep;
mod topo_impls;
