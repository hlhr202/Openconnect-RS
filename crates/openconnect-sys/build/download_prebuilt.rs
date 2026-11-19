use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const URL: &str = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-aarch64-apple-darwin.zip?viasf=1";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const URL: &str = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-x86_64-apple-darwin.zip?viasf=1";

#[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "gnu"))]
const URL: &str = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-msys.zip?viasf=1";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const URL: &str = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-linux-x86_64.zip?viasf=1";

pub fn download_prebuilt_from_sourceforge(outdir: &Path) {
    let output = outdir.join("openconnect.zip");
    let mut response = reqwest::blocking::get(URL)
        .expect("failed to download prebuilt openconnect archive")
        .error_for_status()
        .expect("prebuilt openconnect download returned HTTP error");
    let mut file = std::fs::File::create(&output).expect("failed to create archive file");
    std::io::copy(&mut response, &mut file).expect("failed to write prebuilt archive");
    drop(file);

    if let Ok(expected_sha256) = std::env::var("OPENCONNECT_PREBUILT_SHA256") {
        let mut file = std::fs::File::open(&output).expect("failed to reopen archive for hashing");
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = file.read(&mut buf).expect("failed reading archive");
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let actual = hex::encode(hasher.finalize());
        assert_eq!(
            actual.to_lowercase(),
            expected_sha256.trim().to_lowercase(),
            "SHA-256 mismatch for downloaded prebuilt openconnect archive"
        );
    }

    let file = std::fs::File::open(&output).expect("failed to open archive");
    let mut archive = zip::ZipArchive::new(file).expect("failed to parse zip archive");
    archive.extract(outdir).expect("failed to extract archive");
    std::fs::remove_file(output).expect("failed to remove temporary zip");
}
