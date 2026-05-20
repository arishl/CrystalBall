use std::fs;
use std::io::{self, Read};
use std::path::Path;

use crate::file_entry::is_markdown;

pub const PREVIEW_LIMIT: usize = 4_000;
pub const TAB_LIMIT: usize = 1_000_000;

#[derive(Clone)]
pub struct TextDocument {
    pub text: String,
    pub kind: TextKind,
    pub truncated: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    Code(CodeLanguage),
    Markdown,
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeLanguage {
    C,
    Cpp,
    Makefile,
    Python,
    Rust,
}

pub fn read_preview(path: &Path) -> io::Result<Option<TextDocument>> {
    read_text_document(path, PREVIEW_LIMIT)
}

pub fn read_tab_document(path: &Path) -> io::Result<Option<TextDocument>> {
    read_text_document(path, TAB_LIMIT)
}

fn read_text_document(path: &Path, limit: usize) -> io::Result<Option<TextDocument>> {
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(limit + 1);
    file.by_ref()
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;

    if !looks_like_text(&bytes) {
        return Ok(None);
    }

    let truncated = bytes.len() > limit;
    bytes.truncate(limit);

    let Ok(mut text) = String::from_utf8(bytes) else {
        return Ok(None);
    };

    if truncated {
        text.push_str("\n\n...");
    }

    Ok(Some(TextDocument {
        text,
        kind: if let Some(language) = code_language_from_path(path) {
            TextKind::Code(language)
        } else if is_markdown(path) {
            TextKind::Markdown
        } else {
            TextKind::Plain
        },
        truncated,
    }))
}

fn looks_like_text(bytes: &[u8]) -> bool {
    !bytes.iter().any(|byte| *byte == 0)
}

pub fn code_language_from_name(name: &str) -> Option<CodeLanguage> {
    match name.trim().to_ascii_lowercase().as_str() {
        "c" => Some(CodeLanguage::C),
        "cc" | "cpp" | "c++" | "cxx" | "h++" | "hh" | "hpp" | "hxx" => Some(CodeLanguage::Cpp),
        "make" | "makefile" | "mk" => Some(CodeLanguage::Makefile),
        "py" | "python" | "python3" => Some(CodeLanguage::Python),
        "rs" | "rust" => Some(CodeLanguage::Rust),
        _ => None,
    }
}

fn code_language_from_path(path: &Path) -> Option<CodeLanguage> {
    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
        if let Some(language) = code_language_from_name(file_name) {
            return Some(language);
        }
    }

    let extension = path.extension()?.to_str()?;
    code_language_from_name(extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_makefiles_by_name_extension_and_fence() {
        assert_eq!(
            code_language_from_name("make"),
            Some(CodeLanguage::Makefile)
        );
        assert_eq!(
            code_language_from_name("makefile"),
            Some(CodeLanguage::Makefile)
        );
        assert_eq!(
            code_language_from_path(Path::new("Makefile")),
            Some(CodeLanguage::Makefile)
        );
        assert_eq!(
            code_language_from_path(Path::new("rules.mk")),
            Some(CodeLanguage::Makefile)
        );
    }
}
