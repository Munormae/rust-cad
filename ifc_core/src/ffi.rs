// src/ffi.rs
use autocxx::include_cpp;

include_cpp! {
    #include "cpp/ifc_shim.hpp"
    safety!(unsafe)
    // НИЧЕГО НЕ generate! — просто проверяем, что заголовки проходят
}

pub fn touch() {} // чтобы crate не был пустой