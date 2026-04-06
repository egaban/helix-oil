# CLAUDE.md

## Project overview

This is a fork of [Helix](https://github.com/helix-editor/helix), a terminal-based text editor written in Rust, with an added oil.nvim-like directory editing feature.

## Crate structure

- **helix-core**: Text primitives -- `Rope`, `Transaction`, `ChangeSet`, `Selection`, `Range`, syntax highlighting via tree-sitter. No UI awareness.
- **helix-view**: Editor state -- `Editor`, `Document`, `View`, `Tree` (split layout). The `Editor` struct is the central state object holding all documents, views, config, LSP/DAP registries, and diagnostics.
- **helix-term**: Terminal UI layer -- commands, keymaps, compositor, event loop. This is where user-facing logic lives.
- **helix-tui**: Low-level terminal rendering (buffer, surface, widgets). Fork of `tui-rs`.
- **helix-lsp** / **helix-dap**: Language server and debug adapter protocol clients.
- **helix-loader**: Config/theme/grammar loading from disk.
- **helix-event**: Event dispatching system.
- **helix-vcs**: Git diff integration.
- **helix-stdx**: Standard library extensions (path normalization, rope helpers, etc).

## Key architectural patterns

### Command system
- Commands are `fn(cx: &mut Context)` functions registered via the `static_commands!` macro in `helix-term/src/commands.rs` (~line 305).
- `Context` gives access to `&mut Editor`, `&mut Jobs`, registers, and count.
- Typable commands (`:w`, `:q`, etc.) are in `helix-term/src/commands/typed.rs`.
- Access current view+doc with macros: `current!(cx.editor)`, `current_ref!(cx.editor)`, `doc!(cx.editor)`, `doc_mut!(cx.editor, &id)`, `view!(cx.editor)`, `view_mut!(cx.editor)`.

### Document / Buffer
- `Document` (`helix-view/src/document.rs`) wraps a `Rope` with selections, undo history, path, syntax, LSP state.
- Scratch buffers have `path: None`, display name `[scratch]`.
- All edits go through `Transaction` -> `doc.apply(&transaction, view_id)`.
- Save pipeline: `write_impl()` (typed.rs) -> `Editor::save()` (editor.rs) -> `Document::save()` -> async file I/O.

### Keymaps
- Default keymap defined in `helix-term/src/keymap/default.rs` via the `keymap!` macro.
- Keymaps are NOT context-dependent -- they're purely key-based. Buffer-specific behavior must be handled inside the command function itself (check doc state and branch).
- Special key names: `"minus"` for `-`, `"ret"` for Enter, `"space"` for space, `"esc"` for escape.

### UI / Compositor
- `Compositor` holds a stack of `Box<dyn Component>` layers.
- `EditorView` is the main component. Overlays (pickers, prompts) are pushed on top.
- Event flow: `Terminal Event -> Application::handle_terminal_events() -> Compositor::handle_event() -> EditorView -> keymap lookup -> command execution`.

### File explorer
- `directory_content()` in `helix-term/src/ui/mod.rs` reads a directory using `ignore::WalkBuilder` with `max_depth(1)`, respecting `.gitignore`, hidden file settings, etc.
- `file_explorer()` wraps this in a `Picker` overlay. The oil feature reuses `directory_content()` directly.

## Oil feature (fork addition)

### What it does
Press `-` in normal mode to open the parent directory as an editable buffer. Edit filenames to rename, delete lines to delete files, add lines to create files. `:w` applies changes. Enter opens files or navigates into directories.

### Files
- `helix-view/src/oil.rs` -- Data model (`OilEntryId`, `OilEntry`, `OilBufferState`), line formatting/parsing, nerd font icon mapping, `build_oil_buffer()`.
- `helix-view/src/editor.rs` -- `oil_buffers: HashMap<DocumentId, OilBufferState>` field on `Editor`. Cleanup in `close_document()`.
- `helix-term/src/commands.rs` -- `oil_open` and `oil_enter` command implementations.
- `helix-term/src/commands/typed.rs` -- `oil_write_impl()` for filesystem operations, intercepted at the top of `write_impl()`.
- `helix-term/src/keymap/default.rs` -- `"minus" => oil_open`, `"ret" => oil_enter` in normal mode.

### How identity tracking works
Each line is formatted as ` <icon><filename><invisible-id>`. The entry ID is binary-encoded using zero-width Unicode characters appended after the filename (see constants in `oil.rs`). This is invisible in the terminal but survives edits, allowing rename detection. Lines without a trailing ID are treated as new files. `parse_oil_line_name()` strips both the hidden ID and the visible nerd font icon to extract the bare filename.

### How save works
`oil_write_impl()` diffs the current buffer against the original snapshot stored in `OilBufferState`:
- IDs present with changed name -> rename (`fs::rename`)
- IDs missing from buffer -> delete (`fs::remove_file` / `fs::remove_dir_all`)
- Lines without IDs -> create (`fs::File::create` / `fs::create_dir_all`). Brace expansion is supported (e.g. `foo.{c,h}` creates both `foo.c` and `foo.h`).
- Order: renames first, creates second, deletes last (safe ordering)
After applying, the buffer is refreshed with the new directory listing.

## Build & test

```sh
cargo build          # full build
cargo check          # type-check only (faster)
cargo test -p helix-view -- oil   # run oil-specific tests
```

The binary lands in `target/debug/hx`.
