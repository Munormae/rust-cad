use std::{env, fs, path::PathBuf};

fn main() {
    // Пересобираем при изменениях
    println!("cargo:rerun-if-changed=ifcparse/IfcFile.cpp");
    println!("cargo:rerun-if-changed=ifcparse/IfcFile.h");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 1) Патч-хедер (класть в OUT_DIR, не в репо)
    let patch = out_dir.join("patch_undef_status.h");
    fs::write(
        &patch,
        b"#ifdef Status\n#  undef Status\n#endif\n#ifdef STATUS\n#  undef STATUS\n#endif\n",
    )
        .expect("write patch_undef_status.h");
    let patch_abs = dunce::canonicalize(&patch).unwrap().display().to_string();

    // 2) Фейковые заголовки rocksdb/* (на случай безусловных include'ов)
    let fake_root = out_dir.join("fake_rocksdb").join("rocksdb");
    fs::create_dir_all(&fake_root).expect("create fake rocksdb dir");
    fs::write(fake_root.join("table.h"), b"#pragma once\n").unwrap();
    fs::write(fake_root.join("status.h"), b"#pragma once\n").unwrap();

    // 3) Настройка компиляции C++
    let mut cc = cc::Build::new();
    cc.cpp(true);

    // include-пути: сперва наши фейки, затем исходники и boost
    cc.include(out_dir.join("fake_rocksdb"))
        .include("ifcparse")
        .include("cpp");

    if let Ok(b) = env::var("BOOST_INCLUDEDIR") {
        eprintln!("ifc_core@0.1.0: BOOST_INCLUDEDIR = {}", b);
        cc.include(b);
    } else {
        // подставь свой путь к boost при необходимости
        cc.include(r"C:\local\boost_1_89_0");
    }

    // Флаги под MSVC / GCC+Clang
    if cfg!(target_env = "msvc") {
        cc.flag("/std:c++17")
            .flag("/Zc:__cplusplus")
            .flag("/EHsc")
            .flag("/bigobj")
            .flag("/W4")
            .flag("/showIncludes")
            .flag("/FC")
            .flag("/UStatus")
            .flag("/USTATUS")
            // Форс-инклюдим АБСОЛЮТНЫЙ путь к нашему патчу в OUT_DIR
            .flag(&format!("/FI\"{}\"", patch_abs))
            .define("NOMINMAX", None)
            .define("WIN32_LEAN_AND_MEAN", None)
            .define("NOGDI", None)
            .define("IFCOPENSHELL_HAVE_ROCKSDB", "0")
            .define("HAVE_ROCKSDB", "0");
    } else {
        cc.flag("-std=gnu++17")
            .flag("-fPIC")
            .flag("-Wextra")
            .flag("-include")
            .flag(&patch_abs)
            .define("IFCOPENSHELL_HAVE_ROCKSDB", "0")
            .define("HAVE_ROCKSDB", "0");
    }

    // 4) Для теста собираем только один файл
    cc.file("ifcparse/IfcFile.cpp").compile("ifc_schemas");

    println!("cargo:rustc-link-lib=static=ifc_schemas");
}
