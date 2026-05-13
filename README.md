# CrystalBall

CrystalBall is a compact native file explorer, editor, and terminal for working inside local projects without constantly switching tools.

It is built in Rust with `egui` and `eframe`.

## Features

- File tree with expandable folders, hover previews, and subtle Git status colors.
- Clickable path breadcrumbs and a Copy Path action for quick navigation.
- Multi-tab text editor with dirty-file prompts, Save/Revert controls, line numbers, and in-file search.
- Syntax highlighting for Rust, Python, C, and C++.
- Markdown preview support with highlighted fenced code blocks.
- Fuzzy Quick Open with `Cmd+P` / `Ctrl+P`.
- Folder search for quickly finding files by name.
- Git Helper window with a Markdown-style command reference.
- Persistent Markdown notes saved in `~/.crystalball/notes.md`.
- One-click starter project generation for Rust and C++ Make projects.
- Integrated terminal that follows the current explorer directory.
- Terminal history, `cd` handling, `clear`, and path completion with Tab.
- Project Memory Trails for recently opened files, folders, saves, and commands.
- Installed builds can automatically install newer GitHub release artifacts.

## Run Locally

```bash
cargo run
```

## Build

```bash
cargo build --release
```

The optimized binary is written to `target/release/CrystalBall`.

## Generated Projects

Use `New Project` in the toolbar to scaffold a starter project in the current explorer directory.

- Rust projects include `Cargo.toml`, `src/main.rs`, `.gitignore`, and a short README.
- C++ projects include `Makefile`, `src/main.cpp`, `.gitignore`, and a short README.

CrystalBall chooses a non-conflicting directory name such as `rust_project`, `rust_project_2`, or `cpp_project_2`.

## Build the macOS App

On macOS, package the release binary into a `.app` bundle with the CrystalBall icon:

```bash
cargo build --release
scripts/package-macos.sh
```

The app bundle is written to `dist/macos/CrystalBall.app`.

## Release Artifacts

The `Build Artifacts` GitHub Actions workflow builds downloadable packages for:

- Linux: `CrystalBall-linux-x86_64.tar.gz`
- macOS: `CrystalBall-macos-app.tar.gz`
- Windows: `CrystalBall-windows-x86_64.zip`

The workflow runs manually through `workflow_dispatch`, on `v*` tags, and on the nightly schedule for the `nightly` branch. Tagged builds attach the platform artifacts to the GitHub Release.

## Installing

Each artifact includes a small installer:

- Linux and macOS: `install.sh`
- Windows: `install.ps1`

For macOS, extract `CrystalBall-macos-app.tar.gz` and run:

```bash
./install.sh
```

The installer copies `CrystalBall.app` to `/Applications` when possible, otherwise to `~/Applications`.

## Auto Updates

Installed builds check the latest GitHub Release at startup. If the latest `v*` tag is newer than the app's `Cargo.toml` version, CrystalBall downloads the matching platform artifact and runs its installer.

To publish an update:

```bash
# Update the package version in Cargo.toml first.
git add Cargo.toml Cargo.lock
git commit -m "Bump version to 0.3.0"
git tag v0.3.0
git push origin main v0.3.0
```

Users who installed a build before auto updates existed must manually install one updater-enabled release first.

Set `CRYSTALBALL_DISABLE_AUTO_UPDATE=1` to disable update checks.

## CI

The Rust CI workflow runs:

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo build --locked --release
```

Linux CI also installs the GUI system dependencies required by `eframe`.

## Notes

Text previews are bounded so large files do not block the UI. Files that look binary are skipped instead of displayed as text.

The integrated terminal runs commands in the current explorer directory. `cd` changes the explorer directory, and `clear` clears the terminal output.
