use autocxx::prelude::*;

include_cpp! {
    #include "ifc_bridge.hpp"
    safety!(unsafe_ffi)

    generate!("ifcbridge::FileRaw")
    generate_pod!("ifcbridge::Pt2")
    generate_pod!("ifcbridge::ProfileRaw")
    generate_pod!("ifcbridge::ExtrusionRaw")

    generate!("ifcbridge::import_ifc")
    generate!("ifcbridge::extrusions_ptr")
    generate!("ifcbridge::extrusions_len")
}

// Удобные реэкспорты
pub use ffi::ifcbridge::ExtrusionRaw;
pub use ffi::ifcbridge::FileRaw;
pub use ffi::ifcbridge::ProfileRaw;
pub use ffi::ifcbridge::Pt2;
pub use ffi::ifcbridge::extrusions_len;
pub use ffi::ifcbridge::extrusions_ptr;
pub use ffi::ifcbridge::import_ifc;
