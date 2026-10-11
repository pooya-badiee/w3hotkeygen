//! With the `casclib` feature, builds CascLib (vendor/CascLib, MIT) as a
//! static library with CMake, to compare the pure-Rust reader against.

fn main() {
    #[cfg(feature = "casclib")]
    casclib();
}

#[cfg(feature = "casclib")]
fn casclib() {
    use std::path::Path;

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/CascLib");
    println!("cargo:rerun-if-changed={}", src.join("src").display());
    println!("cargo:rerun-if-changed={}", src.join("CMakeLists.txt").display());

    let dst = cmake::Config::new(&src)
        .define("CASC_BUILD_SHARED_LIB", "OFF")
        .define("CASC_BUILD_STATIC_LIB", "ON")
        // Wide-char paths, so any install folder name works.
        .define("CASC_UNICODE", "ON")
        // CascLib asks for CMake 3.2; CMake 4 refuses anything below 3.5.
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        .build();

    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=casc");
}
