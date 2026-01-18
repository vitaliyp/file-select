# file-list

A TUI file selector written in Rust using ratatui and crossterm.

## Architecture

### Modules

- `main.rs` - Entry point, terminal setup, event loop. Writes TUI to `/dev/tty` to keep stdout clean for output.
- `config.rs` - CLI argument parsing with clap derive
- `app.rs` - Application state, key handling, search mode, contains `App` struct, `FocusedPane` enum, and `AppAction` enum
- `ui.rs` - Ratatui rendering, two-pane layout with status bar and legend
- `file_browser.rs` - Directory reading, navigation, `FileEntry` and `BrowserState` structs
- `selection.rs` - Selection management with `HashSet<PathBuf>`, tracks valid and invalid paths separately
- `input.rs` - Stdin path reading for piped input

### Key Design Decisions

- **HashSet<PathBuf>** for O(1) selection lookups, stores canonical paths
- **Separate valid/invalid tracking** in SelectionState - invalid paths (non-existent files) are stored as-is and displayed in red
- **TUI writes to /dev/tty** instead of stdout to allow clean piping of selected paths
- **Dual-pane UI** with Tab switching between Files and Selected panes
- **Manual scroll offset tracking** in BrowserState and App for proper list scrolling behavior (cursor at top when moving up, at bottom when moving down)
- **Search mode** with incremental search - jumps to first match starting with query, falls back to contains match
- **AppAction enum** for clean separation of action handling (Continue, Quit, Confirm, Save)

## Building

```bash
cargo build
cargo build --release
```

## Testing Changes

```bash
cargo run                          # Basic usage
cargo run -- --help                # Show options
cargo run -- -H                    # Show hidden files
cargo run -- -a                    # Output absolute paths
cargo run -- -f selections.txt    # Edit a selections file
echo "Cargo.toml" | cargo run      # Pipe pre-selections
```
