use std::env;
use std::path::PathBuf;

macro_rules! print_build_warning {
    ($($arg:tt)*) => {
        println!("cargo:warning={}", format_args!($($arg)*));
    };
}

// mod find_lib;

// TODO: optimize path search
fn main() {
    let link = "static";

    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let openconnect_lib = format!("{}/openconnect/.libs", dir);
    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", openconnect_lib);

    // macOS search path
    #[cfg(target_os = "macos")]
    {
        // the order is important!!!
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-search=/usr/local/lib");
        // TODO: for stdc++, optimize auto search
        println!("cargo:rustc-link-search=/opt/homebrew/opt/llvm/lib/c++");
        println!("cargo:rustc-link-search=/opt/homebrew/opt/libiconv/lib");
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=/opt/local/lib");
    }

    // Linux search path
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-search=/usr/local/lib");
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-search=/usr/lib/x86_64-linux-gnu");
        // TODO: for stdc++, optimize auto search
        println!("cargo:rustc-link-search=/usr/lib/gcc/x86_64-linux-gnu/11");
    }

    // windows search path
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-search=C:\\msys64\\mingw64\\lib");
        println!(
            "cargo:rustc-link-search=C:\\msys64\\mingw64\\lib\\gcc\\x86_64-w64-mingw32\\13.2.0"
        );

        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        let target_path = out_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        print_build_warning!("target_path: {:?}", target_path.display());
        println!("cargo:rustc-link-search={}", target_path.display());
    }

    #[cfg(not(target_os = "windows"))]
    {
        // link for openssl
        println!("cargo:rustc-link-lib={}=crypto", link);
        println!("cargo:rustc-link-lib={}=ssl", link);

        // link for xml2
        println!("cargo:rustc-link-lib={}=xml2", link);
        println!("cargo:rustc-link-lib={}=z", link);
        println!("cargo:rustc-link-lib={}=lzma", link);
        println!("cargo:rustc-link-lib={}=icui18n", link);
        println!("cargo:rustc-link-lib={}=icudata", link);
        println!("cargo:rustc-link-lib={}=icuuc", link);
    }

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib={}=crypto", link);
        println!("cargo:rustc-link-lib={}=ssl", link);

        // link for xml2
        println!("cargo:rustc-link-lib={}=xml2", link);
        println!("cargo:rustc-link-lib={}=iconv", link);
        println!("cargo:rustc-link-lib={}=z", link);
        println!("cargo:rustc-link-lib={}=lz4", link);
        println!("cargo:rustc-link-lib={}=lzma", link);
        println!("cargo:rustc-link-lib={}=intl", link);
        println!("cargo:rustc-link-lib={}=gcc", link);
        println!("cargo:rustc-link-lib={}=mingw32", link);
        println!("cargo:rustc-link-lib={}=mingwex", link);

        println!("cargo:rustc-link-lib=dylib=wintun")
    }

    // link c++ stdlib
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib={}=stdc++", link);
    }

    #[cfg(target_os = "macos")]
    {
        // link for iconv
        println!("cargo:rustc-link-lib={}=iconv", link);

        // link for lz4
        println!("cargo:rustc-link-lib={}=lz4", link);

        // link for c++ stdlib
        println!("cargo:rustc-link-lib={}=c++", link);
        println!("cargo:rustc-link-lib={}=c++abi", link);
    }

    // Tell cargo to tell rustc to link the openconnect shared library.
    println!("cargo:rustc-link-lib=static=openconnect");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=c-src/helper.h");
    println!("cargo:rerun-if-changed=c-src/helper.c");

    // ===== compile helper.c start =====
    let mut build = cc::Build::new();
    let build = build
        .file("c-src/helper.c")
        .include("c-src")
        .include("openconnect"); // maybe not needed
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
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
