use std::collections::HashMap;
use std::path::{Path, PathBuf};

use helix_core::Rope;

/// Unique identifier for a filesystem entry within an oil buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OilEntryId(pub u64);

/// Represents an original filesystem entry at the time the oil buffer was created.
#[derive(Debug, Clone)]
pub struct OilEntry {
    pub id: OilEntryId,
    pub original_name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// State for a single oil buffer.
#[derive(Debug, Clone)]
pub struct OilBufferState {
    /// The directory this oil buffer represents.
    pub directory: PathBuf,
    /// The original entries at the time the buffer was opened, keyed by id.
    pub original_entries: HashMap<OilEntryId, OilEntry>,
    /// Counter for generating unique entry IDs.
    next_id: u64,
}

impl OilBufferState {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            original_entries: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add_entry(&mut self, name: String, path: PathBuf, is_dir: bool) -> OilEntryId {
        let id = OilEntryId(self.next_id);
        self.next_id += 1;
        self.original_entries.insert(
            id,
            OilEntry {
                id,
                original_name: name,
                path,
                is_dir,
            },
        );
        id
    }
}

// Zero-width space used as delimiter for entry IDs in buffer text.
const ZWS: char = '\u{200B}';

/// Get a nerd font icon for a file based on its extension, or for a directory.
fn file_icon(name: &str, is_dir: bool) -> &'static str {
    if is_dir {
        return "\u{f024b} "; // 󰉋 directory icon
    }

    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        // Rust
        "rs" => "\u{e7a8} ",        //
        // Python
        "py" => "\u{e73c} ",        //
        "pyi" => "\u{e73c} ",
        // JavaScript / TypeScript
        "js" => "\u{e74e} ",        //
        "mjs" => "\u{e74e} ",
        "cjs" => "\u{e74e} ",
        "ts" => "\u{e628} ",        //
        "tsx" => "\u{e7ba} ",       //
        "jsx" => "\u{e7ba} ",
        // Web
        "html" | "htm" => "\u{e736} ", //
        "css" => "\u{e749} ",       //
        "scss" | "sass" => "\u{e749} ",
        "svg" => "\u{f0721} ",      // 󰜡
        // Shell
        "sh" | "bash" | "zsh" | "fish" => "\u{e795} ", //
        // C / C++
        "c" => "\u{e61e} ",         //
        "h" => "\u{e61e} ",
        "cpp" | "cc" | "cxx" => "\u{e61d} ", //
        "hpp" | "hxx" => "\u{e61d} ",
        // Go
        "go" => "\u{e626} ",        //
        // Java / Kotlin
        "java" => "\u{e738} ",      //
        "kt" | "kts" => "\u{e634} ", //
        // Ruby
        "rb" => "\u{e739} ",        //
        // PHP
        "php" => "\u{e73d} ",       //
        // Lua
        "lua" => "\u{e620} ",       //
        // Markdown / Docs
        "md" | "mdx" => "\u{e73e} ", //
        "txt" => "\u{f0219} ",      // 󰈙
        "pdf" => "\u{f0226} ",      // 󰈦
        // Config / Data
        "json" => "\u{e60b} ",      //
        "yaml" | "yml" => "\u{e6a8} ", //
        "toml" => "\u{e6b2} ",      //
        "xml" => "\u{f05c0} ",      // 󰗀
        "csv" => "\u{f0219} ",
        // Git
        "gitignore" | "gitmodules" | "gitattributes" => "\u{e702} ", //
        // Docker
        "dockerfile" => "\u{e7b0} ", //
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" => "\u{f021a} ", // 󰈚
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "\u{f0187} ", // 󰆇
        // Lock files
        "lock" => "\u{f023} ",      //
        // Nix
        "nix" => "\u{f313} ",       //
        // Elixir / Erlang
        "ex" | "exs" => "\u{e62d} ", //
        "erl" | "hrl" => "\u{e7b1} ", //
        // Haskell
        "hs" => "\u{e777} ",        //
        // Zig
        "zig" => "\u{e6a9} ",       //
        // Swift
        "swift" => "\u{e755} ",     //
        // Dart
        "dart" => "\u{e798} ",      //
        // SQL
        "sql" => "\u{e706} ",       //
        // Vim
        "vim" => "\u{e62b} ",       //
        // Default
        _ => match name {
            "Makefile" | "Justfile" => "\u{e779} ", //
            "Dockerfile" => "\u{e7b0} ",
            "LICENSE" => "\u{f0219} ",
            ".env" => "\u{f462} ",   //
            _ => "\u{f0214} ",       // 󰈔 generic file
        },
    }
}

/// Format a single oil buffer line with embedded entry ID and icon.
pub fn format_oil_line(id: OilEntryId, name: &str, is_dir: bool) -> String {
    let icon = file_icon(name, is_dir);
    format!("{}{}{}{}{}\n", ZWS, id.0, ZWS, icon, name)
}

/// Strip the leading icon (if any) from a visible name returned by `parse_oil_line`.
/// Returns the bare filename.
pub fn strip_icon(visible_name: &str) -> &str {
    // Icons are a nerd font codepoint + space at the start.
    // Find the first space that follows non-ASCII chars at the start.
    let trimmed = visible_name.trim_start();
    if let Some(space_pos) = trimmed.find(' ') {
        let prefix = &trimmed[..space_pos];
        // If the prefix is all non-ASCII (icon chars), strip it + the space
        if !prefix.is_empty() && prefix.chars().all(|c| !c.is_ascii()) {
            return trimmed[space_pos + 1..].trim_start();
        }
    }
    trimmed
}

/// Parse a line from an oil buffer.
/// Returns (Option<OilEntryId>, visible_name) where visible_name includes the icon prefix.
pub fn parse_oil_line(line: &str) -> (Option<OilEntryId>, &str) {
    let line = line.trim_end_matches('\n').trim_end_matches('\r');

    if !line.starts_with(ZWS) {
        return (None, line);
    }

    // Skip the first ZWS character
    let after_first_zws = &line[ZWS.len_utf8()..];

    // Find the second ZWS
    if let Some(end_pos) = after_first_zws.find(ZWS) {
        let id_str = &after_first_zws[..end_pos];
        if let Ok(id) = id_str.parse::<u64>() {
            let name_start = end_pos + ZWS.len_utf8();
            let name = &after_first_zws[name_start..];
            return (Some(OilEntryId(id)), name);
        }
    }

    (None, line)
}

/// Parse a line and return the bare filename (icon stripped).
pub fn parse_oil_line_name(line: &str) -> (Option<OilEntryId>, &str) {
    let (id, visible) = parse_oil_line(line);
    (id, strip_icon(visible))
}

/// Build the buffer text and oil state for a directory listing.
pub fn build_oil_buffer(
    directory: &Path,
    entries: Vec<(PathBuf, bool)>,
) -> (Rope, OilBufferState) {
    let mut state = OilBufferState::new(directory.to_path_buf());
    let mut text = String::new();

    for (path, is_dir) in entries {
        let name = path
            .strip_prefix(directory)
            .unwrap_or(&path)
            .to_string_lossy();
        let display_name = if is_dir {
            format!("{}/", name)
        } else {
            name.to_string()
        };
        let id = state.add_entry(display_name.clone(), path, is_dir);
        text.push_str(&format_oil_line(id, &display_name, is_dir));
    }

    if text.is_empty() {
        text.push('\n');
    }

    (Rope::from(text), state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_and_parse_roundtrip() {
        let id = OilEntryId(42);
        let name = "test_file.txt";
        let line = format_oil_line(id, name, false);
        let (parsed_id, parsed_name) = parse_oil_line_name(&line);
        assert_eq!(parsed_id, Some(id));
        assert_eq!(parsed_name, name);
    }

    #[test]
    fn test_format_dir_roundtrip() {
        let id = OilEntryId(7);
        let line = format_oil_line(id, "my_dir/", true);
        let (parsed_id, parsed_name) = parse_oil_line_name(&line);
        assert_eq!(parsed_id, Some(id));
        assert_eq!(parsed_name, "my_dir/");
    }

    #[test]
    fn test_parse_new_entry_no_icon() {
        let (id, name) = parse_oil_line_name("new_file.txt");
        assert_eq!(id, None);
        assert_eq!(name, "new_file.txt");
    }

    #[test]
    fn test_parse_empty_line() {
        let (id, name) = parse_oil_line_name("");
        assert_eq!(id, None);
        assert_eq!(name, "");
    }

    #[test]
    fn test_strip_icon() {
        assert_eq!(strip_icon("\u{e7a8} main.rs"), "main.rs");
        assert_eq!(strip_icon("\u{f024b} src/"), "src/");
        assert_eq!(strip_icon("plain_name.txt"), "plain_name.txt");
        assert_eq!(strip_icon(""), "");
    }

    #[test]
    fn test_file_icons() {
        assert!(file_icon("main.rs", false).contains('\u{e7a8}'));
        assert!(file_icon("index.js", false).contains('\u{e74e}'));
        assert!(file_icon("src", true).contains('\u{f024b}'));
    }
}
