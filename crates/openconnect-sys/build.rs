use std::env;
use std::path::PathBuf;

fn get_lib_path() -> String {
    let path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = path.join("../../..");
    target_dir.to_string_lossy().to_string()
}

fn copy_libs() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let lib_dir = get_lib_path();

    // copy static library
    #[cfg(not(target_os = "windows"))]
    std::fs::copy(
        format!("{}/openconnect/.libs/libopenconnect.a", dir),
        format!("{}/libopenconnect.a", lib_dir),
    )
    .unwrap();

    // copy shared library on linux
    #[cfg(target_os = "linux")]
    {
        std::fs::copy(
            format!("{}/openconnect/.libs/libopenconnect.so", dir),
            format!("{}/libopenconnect.so", lib_dir),
        )
        .unwrap();

        std::fs::copy(
            format!("{}/openconnect/.libs/libopenconnect.so.5", dir),
            format!("{}/libopenconnect.so.5", lib_dir),
        )
        .unwrap();
    }

    // copy shared library on macos
    #[cfg(target_os = "macos")]
    {
        std::fs::copy(
            format!("{}/openconnect/.libs/libopenconnect.dylib", dir),
            format!("{}/libopenconnect.dylib", lib_dir),
        )
        .unwrap();

        std::fs::copy(
            format!("{}/openconnect/.libs/libopenconnect.5.dylib", dir),
            format!("{}/libopenconnect.5.dylib", lib_dir),
        )
        .unwrap();
    }
}

// TODO: check macos
fn main() {
    copy_libs();
    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", get_lib_path());

    // Tell cargo to tell rustc to link the openconnect shared library.
    println!("cargo:rustc-link-lib=dylib=openconnect");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=c-src/helper.h");
    println!("cargo:rerun-if-changed=c-src/helper.c");

    // Compile helper.c
    cc::Build::new()
        .file("c-src/helper.c")
        .include("c-src")
        .include("openconnect") // maybe not needed
        .compile("helper");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .header("c-src/helper.h")
        .clang_arg("-I./openconnect")
        .enable_function_attribute_detection()
        .trust_clang_mangling(true)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
