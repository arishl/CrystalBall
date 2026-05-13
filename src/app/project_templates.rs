use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) enum ProjectKind {
    Rust,
    Cpp,
}

impl ProjectKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Cpp => "C++",
        }
    }
}

pub(crate) fn create_project(root: &Path, kind: ProjectKind) -> io::Result<PathBuf> {
    match kind {
        ProjectKind::Rust => create_rust_project(root),
        ProjectKind::Cpp => create_cpp_project(root),
    }
}

fn create_rust_project(root: &Path) -> io::Result<PathBuf> {
    let project_dir = unique_project_dir(root, "rust_project");
    let package_name = project_name(&project_dir, "rust_project");
    fs::create_dir_all(project_dir.join("src"))?;

    fs::write(
        project_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"

[dependencies]
"#
        ),
    )?;

    fs::write(
        project_dir.join("src/main.rs"),
        r#"fn main() {
    println!("Hello from Rust!");
}
"#,
    )?;

    fs::write(project_dir.join(".gitignore"), "/target\n")?;
    fs::write(
        project_dir.join("README.md"),
        "# Rust Project\n\nRun with:\n\n```bash\ncargo run\n```\n",
    )?;

    Ok(project_dir)
}

fn create_cpp_project(root: &Path) -> io::Result<PathBuf> {
    let project_dir = unique_project_dir(root, "cpp_project");
    fs::create_dir_all(project_dir.join("src"))?;

    fs::write(
        project_dir.join("Makefile"),
        r#"CXX := c++
CXXFLAGS := -std=c++20 -Wall -Wextra -pedantic -O2
TARGET := app
SRC := src/main.cpp
BUILD_DIR := build

.PHONY: all run clean

all: $(BUILD_DIR)/$(TARGET)

$(BUILD_DIR)/$(TARGET): $(SRC)
	mkdir -p $(BUILD_DIR)
	$(CXX) $(CXXFLAGS) $< -o $@

run: all
	./$(BUILD_DIR)/$(TARGET)

clean:
	rm -rf $(BUILD_DIR)
"#,
    )?;

    fs::write(
        project_dir.join("src/main.cpp"),
        r#"#include <iostream>

int main() {
    std::cout << "Hello from C++!" << '\n';
    return 0;
}
"#,
    )?;

    fs::write(project_dir.join(".gitignore"), "/build\n")?;
    fs::write(
        project_dir.join("README.md"),
        "# C++ Project\n\nBuild and run with:\n\n```bash\nmake run\n```\n",
    )?;

    Ok(project_dir)
}

fn unique_project_dir(root: &Path, base_name: &str) -> PathBuf {
    let mut candidate = root.join(base_name);
    let mut suffix = 2;

    while candidate.exists() {
        candidate = root.join(format!("{base_name}_{suffix}"));
        suffix += 1;
    }

    candidate
}

fn project_name(project_dir: &Path, fallback: &str) -> String {
    project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.replace('-', "_"))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| fallback.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{ProjectKind, create_project};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn creates_non_overwriting_project_directories() {
        let root = std::env::temp_dir().join(format!(
            "crystalball-project-template-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp test root");

        let first = create_project(&root, ProjectKind::Rust).expect("create first rust project");
        let second = create_project(&root, ProjectKind::Rust).expect("create second rust project");
        let cpp = create_project(&root, ProjectKind::Cpp).expect("create cpp project");

        assert_ne!(first, second);
        assert!(first.join("Cargo.toml").exists());
        assert!(second.join("Cargo.toml").exists());
        assert!(cpp.join("Makefile").exists());

        fs::remove_dir_all(root).expect("remove temp test root");
    }
}
