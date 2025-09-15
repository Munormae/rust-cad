use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=include/ifc_bridge.hpp");
    println!("cargo:rerun-if-changed=cpp/ifc_bridge.cpp");
    println!("cargo:rerun-if-changed=src/bridge.rs");

    // autocxx + наш cpp
    let mut b = autocxx_build::Builder::new("src/bridge.rs", &["include"])
        .extra_clang_args(&[
            "-std=c++17",
            "-DNOMINMAX", "-DWIN32_LEAN_AND_MEAN", "-DNOGDI",
            &format!("-I{}", PathBuf::from("cpp").display()),
        ])
        .build()
        .expect("autocxx build failed");

    b.file("cpp/ifc_bridge.cpp")
        .flag_if_supported("/Zc:__cplusplus")
        .flag_if_supported("/EHsc")
        .flag_if_supported("/bigobj");
    b.compile("ifc_core_autocxx");
}
