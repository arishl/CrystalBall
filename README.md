# CrystalBall

CrystalBall is a lightweight Rust desktop file explorer and editor built with `egui` and `eframe`.

It is designed for fast local project browsing: open folders, preview files, edit text, run terminal commands, and search without leaving one compact native window.

## Features

- Folder tree explorer with expandable directories.
- Single-click files to open them in editor tabs.
- Dirty-file close confirmation for unsaved changes.
- Editable text files with Save and Revert controls.
- Line numbers in the editor.
- Syntax highlighting for Rust, Python, C, and C++.
- Markdown previews with highlighted fenced code blocks.
- Hover previews for text files.
- Fuzzy quick open with `Cmd+P` / `Ctrl+P`.
- Find in the current file.
- Find files in the current folder.
- Integrated terminal in the active explorer directory.
- Terminal command history with Up and Down arrows.
- Terminal path completion with Tab and a completion preview.
- Collapsible terminal for more editing space.
- Project Memory Trails for jumping back to recent files, folders, saves, and commands.
- Black, gray, and white application chrome with colored syntax highlighting.

## Run Locally

```bash
cargo run
```

## Build

```bash
cargo build --release
```

The release binary is created in `target/release/`.

## Downloadable Builds

GitHub Actions builds release artifacts for:

- Linux
- macOS
- Windows

The artifact workflow runs:

- on demand with `workflow_dispatch`
- on pushes to `main` that change code or build files
- nightly, only when recent code changes are detected
- on `v*` tags, attaching artifacts to the GitHub release

Each artifact includes the binary, this README, and simple install scripts:

- `install.sh` for Unix-like systems
- `install.ps1` for Windows

## CI

The CI workflow runs:

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo build --locked --release
```

Linux CI also installs the GUI system dependencies needed by `eframe`.

## Notes

Text previews are bounded so large files do not block the UI. Files that look binary are skipped instead of displayed as text.

The terminal runs commands in the current explorer directory. `cd` changes the explorer directory, and `clear` clears the terminal output.
