// ifc_core/build.rs
use std::{env, fs, path::PathBuf};

fn main() {
    // Пересборка при изменениях исходников
    println!("cargo:rerun-if-changed=ifcparse/IfcFile.cpp");
    println!("cargo:rerun-if-changed=ifcparse/IfcFile.h");

    // Генерируем заголовок, который гасит конфликтные макросы
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let patch = out_dir.join("patch_undef_status.h");
    fs::write(
        &patch,
        br#"
// Undef Windows/GDI macros if defined
#ifdef Status
#  undef Status
#endif
#ifdef STATUS
#  undef STATUS
#endif
"#,
    )
        .expect("failed to write patch_undef_status.h");

    // Ищем Boost/vcpkg includes
    let boost_inc = env::var("BOOST_INCLUDEDIR")
        .map(PathBuf::from)
        .ok()
        .filter(|p| p.join("boost").join("shared_ptr.hpp").exists())
        .unwrap_or_else(|| {
            for cand in [
                r"C:\vcpkg\installed\x64-windows\include",
                r"C:\local\boost_1_89_0",
                r"C:\local\boost_1_88_0",
            ] {
                let p = PathBuf::from(cand);
                if p.join("boost").join("shared_ptr.hpp").exists() {
                    return p;
                }
            }
            panic!("Boost headers not found. Set BOOST_INCLUDEDIR to your Boost include folder.");
        });

    println!("cargo:warning=Using Boost includes at {}", boost_inc.display());

    // Путь к vcpkg lib рядом с include (для стандартной раскладки vcpkg)
    let vcpkg_lib = boost_inc.parent().map(|p| p.join("lib")).unwrap_or_else(|| {
        PathBuf::from(r"C:\vcpkg\installed\x64-windows\lib")
    });

    // Сообщаем Cargo, где искать либы и что линковать
    println!("cargo:rustc-link-search=native={}", vcpkg_lib.display());
    // RocksDB (динамическая из x64-windows). Если у тебя статический триплет, замени на static=rocksdb.
    println!("cargo:rustc-link-lib=rocksdb");
    // Часто требуются эти системные либы на Windows
    println!("cargo:rustc-link-lib=shlwapi");
    println!("cargo:rustc-link-lib=rpcrt4");
    println!("cargo:rustc-link-lib=bcrypt");
    // Если при линковке появятся unresolved по zlib/lz4/zstd/snappy — установи фичи в vcpkg
    // и добавь (раскомментируй) нужные строки ниже:
    // println!("cargo:rustc-link-lib=zlib");
    // println!("cargo:rustc-link-lib=lz4");
    // println!("cargo:rustc-link-lib=zstd");
    // println!("cargo:rustc-link-lib=snappy");

    // Сборка C++ части
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .files(["ifcparse/IfcFile.cpp"])
        // инклюды проекта
        .include("ifcparse")
        .include("cpp")
        // vcpkg/boost/rocksdb инклюды
        .include(&boost_inc)
        .warnings(true)
        .flag("/std:c++17")
        .flag("/Zc:__cplusplus")
        .flag("/EHsc")
        .flag("/bigobj")
        .define("NOMINMAX", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .define("NOGDI", None)
        // ВКЛЮЧАЕМ поддержку RocksDB (это убирает самодельные типы в headers проекта)
        .define("IFCOPENSHELL_HAVE_ROCKSDB", Some("1"))
        .define("HAVE_ROCKSDB", Some("1"))
        // на всякий случай гасим возможные макросы
        .flag("/UStatus")
        .flag("/USTATUS")
        // форс-инклуд нашего патча
        .flag(&format!("/FI{}", dunce::simplified(&patch).display()));

    build.compile("ifc_core_cpp");
}
