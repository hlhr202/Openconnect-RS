#![allow(unused_imports)]

#[cfg(feature = "native")]
mod download_prebuilt;
#[cfg(feature = "native")]
mod lib_prob;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(openconnect_generated_bindings)");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=c-src/helper.h");
    println!("cargo:rerun-if-changed=c-src/helper.c");
    println!("cargo:rerun-if-changed=c-src/mock_openconnect.h");
    println!("cargo:rerun-if-changed=c-src/mock_openconnect.c");
    println!("cargo:rerun-if-changed=scripts/nix.sh");
    println!("cargo:rerun-if-env-changed=OPENCONNECT_USE_PREBUILT");
    println!("cargo:rerun-if-env-changed=OPENCONNECT_PREBUILT_SHA256");

    // Compile helper.c unconditionally (except docs.rs) so both native and mock-ffi link it.
    if env::var("DOCS_RS").is_ok() {
        return;
    }

    let mut helper_build = cc::Build::new();
    helper_build
        .file("c-src/helper.c")
        .include("c-src")
        .warnings(true)
        .compile("helper");

    if env::var_os("CARGO_FEATURE_MOCK_FFI").is_some() {
        let mut mock_build = cc::Build::new();
        mock_build
            .file("c-src/mock_openconnect.c")
            .include("c-src")
            .warnings(true)
            .compile("mock_openconnect");
        return;
    }

    #[cfg(feature = "native")]
    build_native();

    #[cfg(not(feature = "native"))]
    panic!(
        "openconnect-sys requires either the `native` feature (default) or the `mock-ffi` feature"
    );
}

#[cfg(feature = "native")]
fn build_native() {
    use download_prebuilt::download_prebuilt_from_sourceforge;
    use lib_prob::*;

    let use_prebuilt = env::var("OPENCONNECT_USE_PREBUILT").as_deref() == Ok("true");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let openconnect_src_dir = out_path.join("openconnect");

    let openconnect_lib = openconnect_src_dir.join(".libs");
    let static_lib = openconnect_lib.join("libopenconnect.a");

    if !static_lib.exists() {
        if use_prebuilt {
            download_prebuilt_from_sourceforge(&out_path);
        } else {
            let script = manifest_dir.join("scripts/nix.sh");
            let output = std::process::Command::new("sh")
                .args([
                    script.to_str().expect("invalid script path"),
                    openconnect_src_dir
                        .to_str()
                        .expect("invalid openconnect dir"),
                ])
                .output()
                .expect("failed to execute scripts/nix.sh");
            if !output.status.success() {
                panic!(
                    "scripts/nix.sh failed with status {}:\nstdout:\n{}\nstderr:\n{}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }

    println!(
        "cargo:rustc-link-search={}",
        openconnect_lib.to_str().expect("invalid lib path")
    );
    println!("cargo:rustc-link-lib=static=openconnect");

    #[cfg(target_os = "windows")]
    {
        resolve_mingw64_lib_path();

        let target_path = out_path.ancestors().nth(3).expect("target path not found");
        println!("cargo:rustc-link-search={}", target_path.to_string_lossy());

        let wintun_dll_source = manifest_dir.join("wintun.dll");
        let wintun_dll_target = target_path.join("wintun.dll");
        if wintun_dll_source.exists() {
            std::fs::copy(wintun_dll_source, wintun_dll_target).expect("failed to copy wintun.dll");
        }

        try_pkg_config(&["libxml-2.0", "zlib", "liblz4", "iconv"], true);
        println!("cargo:rustc-link-lib=static=intl");
        println!("cargo:rustc-link-lib=static=crypto");
        println!("cargo:rustc-link-lib=static=ssl");
        println!("cargo:rustc-link-lib=dylib=wintun");
    }

    #[cfg(target_os = "linux")]
    {
        try_pkg_config(
            &["openssl", "libxml-2.0", "zlib", "liblz4", "liblzma"],
            false,
        );
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-search=/usr/local/lib");
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=/usr/local/opt/gettext/lib");
        println!("cargo:rustc-link-search=/opt/homebrew/opt/gettext/lib");

        try_pkg_config(&["openssl", "libxml-2.0", "zlib", "liblz4"], true);

        let intl_candidates = [
            "/usr/local/opt/gettext/lib/libintl.a",
            "/opt/homebrew/opt/gettext/lib/libintl.a",
            "/usr/local/lib/libintl.a",
            "/opt/homebrew/lib/libintl.a",
        ];
        if intl_candidates
            .iter()
            .any(|p| std::path::Path::new(p).exists())
        {
            println!("cargo:rustc-link-lib=static=intl");
        }
        println!("cargo:rustc-link-lib=dylib=iconv");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=dylib=c++abi");
    }

    // Generate bindings into OUT_DIR (never mutate checked-in src/ files during cargo build).
    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .header("c-src/helper.h")
        .clang_arg(format!(
            "-I{}",
            openconnect_src_dir
                .to_str()
                .expect("invalid openconnect src dir")
        ))
        .wrap_unsafe_ops(true)
        .enable_function_attribute_detection()
        .trust_clang_mangling(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    #[cfg(all(target_os = "windows", target_env = "gnu"))]
    let builder = {
        let mingw_root = get_cygpath("/mingw64");
        let mingw_include = get_cygpath("/mingw64/include");
        let mingw_target_include = get_cygpath("/mingw64/x86_64-w64-mingw32/include");
        builder
            .clang_arg(format!("--sysroot={}", mingw_root))
            .clang_arg(format!("-isystem{}", mingw_include))
            .clang_arg(format!("-isystem{}", mingw_target_include))
    };

    let bindings = builder.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings into OUT_DIR");
    println!("cargo:rustc-cfg=openconnect_generated_bindings");
}
