use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct NotesState {
    pub(crate) open: bool,
    pub(crate) text: String,
    path: PathBuf,
}

impl NotesState {
    pub(crate) fn load() -> Self {
        let path = notes_path();
        let text = fs::read_to_string(&path).unwrap_or_default();

        Self {
            open: false,
            text,
            path,
        }
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.path, &self.text)
    }
}

fn notes_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".crystalball")
        .join("notes.md")
}
