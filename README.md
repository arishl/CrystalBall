# CrystalBall

CrystalBall is a small Rust desktop file explorer built with `egui`/`eframe`.

## Features

- Browse the current working directory.
- Double-click folders to enter them.
- Use `Up` to move to the parent folder.
- Hover over text files to see a preview.
- Markdown files (`.md` and `.markdown`) render as Markdown in previews and tabs.
- Python, C, C++, and Rust files render with syntax highlighting.
- Markdown fenced code blocks render with syntax highlighting when tagged as `python`, `c`, `cpp`, `c++`, or `rust`.
- Single-click a file to open it in the tab area.
- Run shell commands in the bottom terminal.
- Drag the top edge of the terminal to resize it.

## Run

```bash
cargo run
```

## Notes

Text previews are intentionally bounded so large files do not block the UI. Binary-looking files are skipped instead of being displayed as text.

The terminal runs commands in the current explorer directory. `cd` changes the explorer directory, and `clear` clears the terminal output. Terminal height is kept at the size you last dragged it to during the session.
