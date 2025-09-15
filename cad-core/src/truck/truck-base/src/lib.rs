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

pub mod bounding_box;
pub mod cgmath64;
pub mod cgmath_extend_traits;
pub mod ders;
pub mod entry_map;
pub mod hash;
pub mod id;
pub mod newton;
pub mod tolerance;
