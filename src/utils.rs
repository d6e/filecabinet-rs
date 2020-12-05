use std::path::{Path, PathBuf};

// TODO: use async paths
pub fn list_files(path: &PathBuf) -> Vec<String> {
    if !path.exists() {
        return Vec::new();
    }
    path.read_dir()
        .expect("read_dir call failed")
        .map(|x| x.unwrap().path())
        .filter(|x| Path::new(x).is_file())
        .filter(|x| {
            let ext: String = x
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or(String::new());
            ext == "pdf" || ext == "jpg" || ext == "png" || ext == "cocoon"
        })
        .map(|x| x.file_name().unwrap().to_str().unwrap().to_owned())
        .collect()
}
