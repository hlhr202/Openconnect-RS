#[macro_export]
macro_rules! print_build_warning {
    ($($arg:tt)*) => {
        println!("cargo:warning={}", format_args!($($arg)*));
    };
}

pub fn is_msys64_shell() -> bool {
    std::env::var("MSYSTEM").is_ok()
}

pub fn get_cygpath(path: &str) -> String {
    use std::ffi::OsString;

    let cygpath = if is_msys64_shell() {
        OsString::from("cygpath")
    } else {
        let system_drive = std::env::var("SystemDrive").expect("SystemDrive not found");
        let cygpath = format!("{}\\msys64\\usr\\bin\\cygpath", system_drive);
        OsString::from(&cygpath)
    };

    let cygpath_cmd = std::process::Command::new(cygpath)
        .arg("-w")
        .arg(path)
        .output()
        .expect("failed to execute cygpath");

    String::from_utf8(cygpath_cmd.stdout).expect("cygpath output is not utf8")
}

pub fn resolve_mingw64_lib_path() {
    let lib_path = get_cygpath("/mingw64/lib");
    print_build_warning!("mingw64_lib_path: {}", lib_path);
    println!("cargo:rustc-link-search={}", lib_path);
}

pub fn try_pkg_config(libs: Vec<&str>) {
    #[cfg(target_os = "windows")]
    {
        std::env::set_var("PKG_CONFIG", "pkg-config");
        std::env::set_var(
            "PKG_CONFIG_PATH",
            "/mingw64/lib/pkgconfig:/mingw64/share/pkgconfig",
        );
        std::env::set_var("PKG_CONFIG_SYSTEM_INCLUDE_PATH", "/mingw64/include");
        std::env::set_var("PKG_CONFIG_SYSTEM_LIBRARY_PATH", "/mingw64/lib");
    }

    #[cfg(target_os = "macos")]
    {
        // TODO: check this
        // setup pkg-config to be homebrew pkg-config
        std::env::set_var(
            "PKG_CONFIG_PATH",
            "/usr/lib/pkgconfig:/usr/local/lib/pkgconfig:/opt/homebrew/lib/pkgconfig",
        );
    }

    let mut conf = pkg_config::Config::new();

    for lib in libs {
        let result = conf.statik(true).probe(lib);
        if result.is_err() {
            print_build_warning!("{} not found", lib);
        }
    }
}

#[test]
fn test_prob() {
    try_pkg_config(vec!["openssl", "libxml-2.0", "zlib", "liblz4", "iconv"])
}
