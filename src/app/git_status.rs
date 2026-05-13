use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use eframe::egui;

pub(crate) fn read_statuses(current_dir: &Path) -> HashMap<PathBuf, FileStatus> {
    let Some(root) = repository_root(current_dir) else {
        return HashMap::new();
    };

    let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain=v1", "-z"])
        .output()
    else {
        return HashMap::new();
    };

    if !output.status.success() {
        return HashMap::new();
    }

    parse_porcelain_status(&root, &output.stdout)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
    Conflicted,
}

impl FileStatus {
    pub(crate) fn tint(self) -> egui::Color32 {
        match self {
            Self::Modified => egui::Color32::from_rgba_unmultiplied(182, 142, 46, 46),
            Self::Added => egui::Color32::from_rgba_unmultiplied(54, 150, 92, 46),
            Self::Deleted => egui::Color32::from_rgba_unmultiplied(184, 64, 72, 54),
            Self::Untracked => egui::Color32::from_rgba_unmultiplied(80, 132, 196, 42),
            Self::Conflicted => egui::Color32::from_rgba_unmultiplied(174, 70, 190, 64),
        }
    }

    pub(crate) fn text_color(self) -> egui::Color32 {
        match self {
            Self::Modified => egui::Color32::from_rgb(238, 214, 154),
            Self::Added => egui::Color32::from_rgb(174, 232, 196),
            Self::Deleted => egui::Color32::from_rgb(248, 184, 188),
            Self::Untracked => egui::Color32::from_rgb(176, 210, 248),
            Self::Conflicted => egui::Color32::from_rgb(238, 188, 248),
        }
    }

    fn from_porcelain(index_status: char, worktree_status: char) -> Option<Self> {
        if index_status == '?' && worktree_status == '?' {
            return Some(Self::Untracked);
        }

        if matches!(index_status, 'U' | 'A' | 'D') && matches!(worktree_status, 'U' | 'A' | 'D') {
            return Some(Self::Conflicted);
        }

        if index_status == 'D' || worktree_status == 'D' {
            return Some(Self::Deleted);
        }

        if matches!(index_status, 'A' | 'R' | 'C') {
            return Some(Self::Added);
        }

        if matches!(index_status, 'M' | 'T') || matches!(worktree_status, 'M' | 'T') {
            return Some(Self::Modified);
        }

        None
    }
}

fn repository_root(current_dir: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(current_dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!root.is_empty()).then(|| PathBuf::from(root))
}

fn parse_porcelain_status(root: &Path, output: &[u8]) -> HashMap<PathBuf, FileStatus> {
    let mut statuses = HashMap::new();
    let mut records = output.split(|byte| *byte == 0);

    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }

        let index_status = record[0] as char;
        let worktree_status = record[1] as char;
        let status = FileStatus::from_porcelain(index_status, worktree_status);

        let path = relative_path(&record[3..]);
        if matches!(index_status, 'R' | 'C') {
            records.next();
        }

        let Some(status) = status else {
            continue;
        };

        statuses.insert(root.join(path), status);
    }

    statuses
}

fn relative_path(record: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(record).into_owned())
}

#[cfg(test)]
mod tests {
    use super::{FileStatus, parse_porcelain_status};
    use std::path::Path;

    #[test]
    fn parses_common_statuses() {
        let root = Path::new("/repo");
        let statuses = parse_porcelain_status(
            root,
            b" M src/main.rs\0A  src/new.rs\0?? notes.txt\0D  old.txt\0UU conflict.txt\0",
        );

        assert_eq!(
            statuses.get(Path::new("/repo/src/main.rs")),
            Some(&FileStatus::Modified)
        );
        assert_eq!(
            statuses.get(Path::new("/repo/src/new.rs")),
            Some(&FileStatus::Added)
        );
        assert_eq!(
            statuses.get(Path::new("/repo/notes.txt")),
            Some(&FileStatus::Untracked)
        );
        assert_eq!(
            statuses.get(Path::new("/repo/old.txt")),
            Some(&FileStatus::Deleted)
        );
        assert_eq!(
            statuses.get(Path::new("/repo/conflict.txt")),
            Some(&FileStatus::Conflicted)
        );
    }

    #[test]
    fn uses_destination_path_for_renames() {
        let root = Path::new("/repo");
        let statuses = parse_porcelain_status(root, b"R  src/new.rs\0src/old.rs\0");

        assert_eq!(
            statuses.get(Path::new("/repo/src/new.rs")),
            Some(&FileStatus::Added)
        );
        assert!(!statuses.contains_key(Path::new("/repo/src/old.rs")));
    }
}
