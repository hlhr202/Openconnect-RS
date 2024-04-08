mod lib_prob;

use lib_prob::*;
use std::env;
use std::path::PathBuf;

// TODO: optimize path search
fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let openconnect_src_dir = manifest_dir.join("openconnect");

    #[cfg(not(target_os = "windows"))]
    {
        let current_dir = env::current_dir().unwrap();
        let script = current_dir.join("scripts/nix.sh");
        let _ = std::process::Command::new("sh")
            .args([
                script.to_str().unwrap(),
                openconnect_src_dir.to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute process");
    }

    // statically link openconnect
    let openconnect_lib = openconnect_src_dir.join(".libs");
    println!(
        "cargo:rustc-link-search={}",
        openconnect_lib.to_str().unwrap()
    );
    println!("cargo:rustc-link-lib=static=openconnect");

    // windows linking
    #[cfg(target_os = "windows")]
    {
        resolve_mingw64_lib_path();

        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let target_path = out_path.ancestors().nth(3).unwrap();
        print_build_warning!("target_path: {}", target_path.to_string_lossy());
        println!("cargo:rustc-link-search={}", target_path.to_string_lossy());

        let wintun_dll_source = format!("{}/wintun.dll", manifest_dir);
        let wintun_dll_target = format!("{}/wintun.dll", target_path.to_string_lossy());
        std::fs::copy(wintun_dll_source, wintun_dll_target).unwrap();

        try_pkg_config(vec!["openssl", "libxml-2.0", "zlib", "liblz4", "iconv"]);
        println!("cargo:rustc-link-lib=static=intl");
        println!("cargo:rustc-link-lib=dylib=wintun")
    }

    // link c++ stdlib
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-search=/usr/local/lib");
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-search=/usr/lib/x86_64-linux-gnu");
        // TODO: for stdc++, optimize auto search
        println!("cargo:rustc-link-search=/usr/lib/gcc/x86_64-linux-gnu/11");

        // link for openssl
        println!("cargo:rustc-link-lib=static=crypto");
        println!("cargo:rustc-link-lib=static=ssl");

        // link for xml2
        println!("cargo:rustc-link-lib=static=xml2");
        println!("cargo:rustc-link-lib=static=z");
        println!("cargo:rustc-link-lib=static=lzma");
        println!("cargo:rustc-link-lib=static=icui18n");
        println!("cargo:rustc-link-lib=static=icudata");
        println!("cargo:rustc-link-lib=static=icuuc");

        // link for stdc++
        println!("cargo:rustc-link-lib=static=stdc++");
    }

    #[cfg(target_os = "macos")]
    {
        // the order is important!!!
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-search=/usr/local/lib");

        try_pkg_config(vec!["openssl", "libxml-2.0", "zlib", "liblz4"]);

        // link for c++ stdlib
        println!("cargo:rustc-link-lib=static=intl"); // fix for x86 catalina
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=dylib=c++abi");

        // if you want to link c++ stdlib statically, use llvm c++ stdlib
        // println!("cargo:rustc-link-search=/opt/homebrew/opt/llvm/lib/c++");
        // println!("cargo:rustc-link-lib=static=c++");
        // println!("cargo:rustc-link-lib=static=c++abi");
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=c-src/helper.h");
    println!("cargo:rerun-if-changed=c-src/helper.c");

    // ===== compile helper.c start =====
    let mut build = cc::Build::new();
    let build = build
        .file("c-src/helper.c")
        .include("c-src");
        // .include(openconnect_src_dir.to_str().unwrap()); // maybe not needed
    build.compile("helper");
    // ===== compile helper.c end =====

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
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
