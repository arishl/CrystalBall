use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

impl FileEntry {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let metadata = fs::metadata(&path).ok()?;
        let name = path.file_name()?.to_string_lossy().into_owned();

        Some(Self {
            path,
            name,
            is_dir: metadata.is_dir(),
        })
    }
}

pub fn is_markdown(path: &Path) -> bool {
    matches!(extension(path).as_deref(), Some("md" | "markdown"))
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}
