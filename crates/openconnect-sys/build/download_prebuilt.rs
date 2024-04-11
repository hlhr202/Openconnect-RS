use std::path::PathBuf;

#[cfg(target_os = "macos")]
#[cfg(target_arch = "aarch64")]
const URL: &str = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-aarch64-apple-darwin.zip?viasf=1";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
const URL: &str = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-x86_64-apple-darwin.zip?viasf=1";

#[cfg(target_os = "windows")]
#[cfg(target_arch = "x86_64")]
#[cfg(target_env = "gnu")]
const URL: &str = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-msys.zip?viasf=1";

pub fn download_prebuilt_from_sourceforge(outdir: PathBuf) {
    let output = outdir.join("openconnect.zip");
    let mut response = reqwest::blocking::get(URL).unwrap();
    let mut file = std::fs::File::create(&output).unwrap();
    std::io::copy(&mut response, &mut file).unwrap();

    let file = std::fs::File::open(&output).unwrap();

    // unzip
    let mut archive = zip::ZipArchive::new(file).unwrap();
    archive.extract(outdir).unwrap();

    // remove zip file
    std::fs::remove_file(output).unwrap();
}
