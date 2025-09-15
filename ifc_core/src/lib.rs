// ifc_core/src/lib.rs
pub mod bridge;

// Реэкспорт из bridge (там уже есть публичные re-export'ы от autocxx)
pub use crate::bridge::{
    import_ifc,
    extrusions_ptr,
    extrusions_len,
    FileRaw,
    ExtrusionRaw,
    ProfileRaw,
    Pt2,
};