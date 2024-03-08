use std::env;
use std::path::PathBuf;

// TODO: check macos
fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // Tell cargo to look for shared libraries in the specified directory
    println!(
        "cargo:rustc-link-search={}",
        format_args!("{}/openconnect/.libs", dir)
    );


    // macOS search path
    println!("cargo:rustc-link-search=/opt/local/lib");
    println!("cargo:rustc-link-search=/usr/local/lib");
    println!("cargo:rustc-link-search=/usr/lib");
    println!("cargo:rustc-link-search=/opt/homebrew/opt/llvm/lib/c++");
    // macOS search path end

    // Tell cargo to tell rustc to link the openconnect shared library.
    println!("cargo:rustc-link-lib=static=openconnect");

    // link for xml2
    println!("cargo:rustc-link-lib=static=xml2");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=iconv");
    println!("cargo:rustc-link-lib=static=icui18n");
    println!("cargo:rustc-link-lib=static=lzma");
    println!("cargo:rustc-link-lib=static=icudata");
    println!("cargo:rustc-link-lib=static=icuuc");

    // link for openssl
    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=ssl");

    // link for lz4
    println!("cargo:rustc-link-lib=static=lz4");

    // link c++ stdlib
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=static=stdc++");

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=static=c++");
    println!("cargo:rustc-link-lib=static=c++abi");

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
        .clang_arg("-static")
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
