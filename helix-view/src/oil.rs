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

/// Format a single oil buffer line with embedded entry ID.
pub fn format_oil_line(id: OilEntryId, name: &str) -> String {
    format!("{}{}{}{}\n", ZWS, id.0, ZWS, name)
}

/// Parse a line from an oil buffer.
/// Returns (Option<OilEntryId>, visible_name).
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
        text.push_str(&format_oil_line(id, &display_name));
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
        let line = format_oil_line(id, name);
        let (parsed_id, parsed_name) = parse_oil_line(&line);
        assert_eq!(parsed_id, Some(id));
        assert_eq!(parsed_name, name);
    }

    #[test]
    fn test_parse_new_entry() {
        let (id, name) = parse_oil_line("new_file.txt");
        assert_eq!(id, None);
        assert_eq!(name, "new_file.txt");
    }

    #[test]
    fn test_parse_directory_entry() {
        let id = OilEntryId(7);
        let line = format_oil_line(id, "my_dir/");
        let (parsed_id, parsed_name) = parse_oil_line(&line);
        assert_eq!(parsed_id, Some(id));
        assert_eq!(parsed_name, "my_dir/");
    }

    #[test]
    fn test_parse_empty_line() {
        let (id, name) = parse_oil_line("");
        assert_eq!(id, None);
        assert_eq!(name, "");
    }
}
