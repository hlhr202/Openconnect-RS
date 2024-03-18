pub fn try_pkg_config(libs: Vec<&str>) {
    // setup pkg-config to be homebrew pkg-config
    std::env::set_var(
        "PKG_CONFIG_PATH",
        "/usr/lib/pkgconfig:/usr/local/lib/pkgconfig:/opt/homebrew/lib/pkgconfig",
    );

    let mut conf = pkg_config::Config::new();

    for lib in libs {
        let result = conf.statik(true).probe(lib);
        if result.is_err() {
            println!("{} not found", lib);
        }
    }
}

#[test]
fn test_prob() {
    try_pkg_config(vec!["openssl", "libxml-2.0", "zlib", "liblz4"])
}
