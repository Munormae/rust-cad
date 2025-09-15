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

#[macro_export]
#[doc(hidden)]
macro_rules! nonpositive_tolerance {
    ($tol: expr, $minimum: expr) => {
        assert!(
            $tol >= $minimum,
            "tolerance must be no less than {:e}",
            $minimum
        );
    };
    ($tol: expr) => {
        nonpositive_tolerance!($tol, TOLERANCE)
    };
}

pub mod traits;
pub use traits::*;
pub mod algo;
#[cfg(feature = "derive")]
pub use truck_derivers::{
    BoundedCurve, BoundedSurface, Cut, Invertible, ParameterDivision1D, ParameterDivision2D,
    ParametricCurve, ParametricSurface, ParametricSurface3D, SearchNearestParameterD1,
    SearchNearestParameterD2, SearchParameterD1, SearchParameterD2, SelfSameGeometry,
    TransformedM3, TransformedM4,
};
#[cfg(feature = "polynomial")]
pub mod polynomial;
