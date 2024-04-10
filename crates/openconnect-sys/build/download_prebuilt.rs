use std::path::PathBuf;

pub fn download_from_sourceforge(outdir: PathBuf) {
    let url = "https://master.dl.sourceforge.net/project/openconnect-prebuilt/prebuilt-openconnect-msys.zip?viasf=1";
    let output = outdir.join("prebuilt-openconnect-msys.zip");
    let mut response = reqwest::blocking::get(url).unwrap();
    let mut file = std::fs::File::create(&output).unwrap();
    std::io::copy(&mut response, &mut file).unwrap();

    let file = std::fs::File::open(&output).unwrap();

    // unzip
    let mut archive = zip::ZipArchive::new(file).unwrap();
    archive.extract(outdir).unwrap();

    // remove zip file
    std::fs::remove_file(output).unwrap();
}
