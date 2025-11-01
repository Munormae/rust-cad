#![doc = "Cryxtal-base provides the foundational math, tolerance and solver utilities for the geometry kernel."]
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

/// Axis-aligned bounding boxes and helpers for merging geometry.
pub mod bounding_box;
/// Double-precision convenience exports for `cgmath`.
pub mod cgmath64;
/// Extended traits and utilities layered on top of `cgmath`.
pub mod cgmath_extend_traits;
/// High-order derivative containers for curves and surfaces.
pub mod ders;
/// Lazy entry map backed by `HashMap`.
pub mod entry_map;
/// Deterministic pseudo-random generators for numeric seeds.
pub mod hash;
/// Address-based identifiers that integrate with standard collections.
pub mod id;
/// Newton solver and optional parallel helpers.
pub mod newton;
/// Tolerance constants, comparison macros, and traits.
pub mod tolerance;

/// Parallel extensions enabled via `feature = "parallel"`.
#[cfg(feature = "parallel")]
pub mod parallel;
