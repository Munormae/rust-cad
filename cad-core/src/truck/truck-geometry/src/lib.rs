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

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use truck_base::bounding_box::Bounded;

const INCLUDE_CURVE_TRIALS: usize = 100;
const PRESEARCH_DIVISION: usize = 50;

pub mod base {
    pub use truck_base::{
        assert_near, assert_near2, bounding_box::BoundingBox, cgmath64::*, hash, hash::HashGen,
        prop_assert_near, prop_assert_near2, tolerance::*,
    };
    pub use truck_geotrait::*;
}
pub mod decorators;
pub mod errors;
pub mod nurbs;
pub mod specifieds;
pub mod prelude {
    use crate::*;
    pub use decorators::*;
    pub use errors::*;
    pub use nurbs::*;
    pub use specifieds::*;
}
