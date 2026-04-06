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

// Zero-width characters used for invisible ID encoding at the end of each line.
// The ID is binary-encoded using two zero-width chars, preceded by a delimiter:
//   <WJ><bits...>  where each bit is ZWS (0) or ZWNJ (1), MSB first.
const ZWS: char = '\u{200B}'; // Zero Width Space  – binary 0
const ZWNJ: char = '\u{200C}'; // Zero Width Non-Joiner – binary 1
const WJ: char = '\u{2060}'; // Word Joiner – ID start delimiter

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

/// Encode an entry ID as an invisible binary string using zero-width characters.
fn encode_oil_id(id: OilEntryId) -> String {
    let mut s = String::new();
    s.push(WJ);
    let n = id.0;
    if n == 0 {
        s.push(ZWS);
        return s;
    }
    let bits = 64 - n.leading_zeros();
    for i in (0..bits).rev() {
        if (n >> i) & 1 == 1 {
            s.push(ZWNJ);
        } else {
            s.push(ZWS);
        }
    }
    s
}

/// Decode an entry ID from a binary string of zero-width characters.
fn decode_oil_id(s: &str) -> Option<OilEntryId> {
    let mut n: u64 = 0;
    for ch in s.chars() {
        match ch {
            ZWS => n = n.checked_mul(2)?,
            ZWNJ => n = n.checked_mul(2)?.checked_add(1)?,
            _ => return None,
        }
    }
    Some(OilEntryId(n))
}

/// Format a single oil buffer line: ` <icon><name><invisible-id>\n`
pub fn format_oil_line(id: OilEntryId, name: &str, is_dir: bool) -> String {
    let icon = file_icon(name, is_dir);
    let encoded_id = encode_oil_id(id);
    format!(" {}{}{}\n", icon, name, encoded_id)
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
/// The ID is expected at the end of the line after a WJ delimiter: `<visible>\u{2060}<binary-id>`
pub fn parse_oil_line(line: &str) -> (Option<OilEntryId>, &str) {
    let line = line.trim_end_matches('\n').trim_end_matches('\r');

    // Find the WJ delimiter that marks the start of the encoded ID
    if let Some(wj_pos) = line.rfind(WJ) {
        let after_wj = &line[wj_pos + WJ.len_utf8()..];
        if let Some(id) = decode_oil_id(after_wj) {
            let name = &line[..wj_pos];
            return (Some(id), name);
        }
    }

    (None, line)
}

/// Parse a line and return the bare filename (icon stripped).
pub fn parse_oil_line_name(line: &str) -> (Option<OilEntryId>, &str) {
    let (id, visible) = parse_oil_line(line);
    (id, strip_icon(visible))
}

/// Expand brace alternations in a filename, e.g. `foo.{c,h}` → `["foo.c", "foo.h"]`.
/// Only the last path component is checked for braces. If no braces are found,
/// returns a single-element vec with the original name.
pub fn expand_braces(name: &str) -> Vec<String> {
    // Only look at the last path component (after the last '/')
    let (prefix, last) = match name.rfind('/') {
        Some(pos) => (&name[..=pos], &name[pos + 1..]),
        None => ("", name),
    };

    // Match {alternatives} in the last component
    if let Some(open) = last.find('{') {
        if let Some(close) = last[open..].find('}') {
            let close = open + close;
            let before = &last[..open];
            let after = &last[close + 1..];
            let alternatives = &last[open + 1..close];

            return alternatives
                .split(',')
                .map(|alt| format!("{}{}{}{}", prefix, before, alt, after))
                .collect();
        }
    }

    vec![name.to_string()]
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

    #[test]
    fn test_line_starts_with_space() {
        let id = OilEntryId(1);
        let line = format_oil_line(id, "test.rs", false);
        assert_eq!(line.chars().next().unwrap(), ' ', "line must start with a space");
    }

    #[test]
    fn test_no_visible_id_digits() {
        let id = OilEntryId(42);
        let line = format_oil_line(id, "test.rs", false);
        // The line should not contain any visible ASCII digit from the ID
        // (digits in the filename are fine but "42" should not appear)
        let (_, visible) = parse_oil_line(&line);
        assert!(
            !visible.contains("42"),
            "ID digits must not appear in visible name"
        );
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        for n in [0, 1, 2, 7, 42, 255, 1000, u64::MAX] {
            let id = OilEntryId(n);
            let encoded = encode_oil_id(id);
            // strip the leading WJ before decoding
            let decoded = decode_oil_id(&encoded[WJ.len_utf8()..]);
            assert_eq!(decoded, Some(id), "roundtrip failed for {n}");
        }
    }

    #[test]
    fn test_expand_braces_basic() {
        assert_eq!(expand_braces("foo.{c,h}"), vec!["foo.c", "foo.h"]);
    }

    #[test]
    fn test_expand_braces_multiple() {
        assert_eq!(
            expand_braces("foo.{js,test.js,spec.js}"),
            vec!["foo.js", "foo.test.js", "foo.spec.js"]
        );
    }

    #[test]
    fn test_expand_braces_no_braces() {
        assert_eq!(expand_braces("foo.c"), vec!["foo.c"]);
    }

    #[test]
    fn test_expand_braces_with_path_prefix() {
        assert_eq!(
            expand_braces("src/foo.{c,h}"),
            vec!["src/foo.c", "src/foo.h"]
        );
    }

    #[test]
    fn test_expand_braces_single_alt() {
        assert_eq!(expand_braces("foo.{c}"), vec!["foo.c"]);
    }

    #[test]
    fn test_expand_braces_unclosed() {
        assert_eq!(expand_braces("foo.{c,h"), vec!["foo.{c,h"]);
    }
}
