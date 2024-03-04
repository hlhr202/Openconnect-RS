use std::env;
use std::path::PathBuf;

#[cfg(not(target_os = "windows"))]
fn copy_libs() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    std::fs::copy(
        format!("{}/openconnect/.libs/libopenconnect.a", dir),
        format!("{}/libopenconnect.a", out_dir),
    )
    .unwrap();

    std::fs::copy(
        format!("{}/openconnect/.libs/libopenconnect.so", dir),
        format!("{}/libopenconnect.so", out_dir),
    )
    .unwrap();

    std::fs::copy(
        format!("{}/openconnect/.libs/libopenconnect.so.5", dir),
        format!("{}/libopenconnect.so.5", out_dir),
    )
    .unwrap();
}

// TODO: check macos
fn main() {
    copy_libs();
    let out_dir = env::var("OUT_DIR").unwrap();
    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", out_dir);

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=dylib=openconnect");
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
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
