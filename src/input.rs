use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;

/// Strip ANSI escape sequences from a string and return the clean content.
/// If the line contains escape sequences, extract the last clean segment
/// (which is typically the actual path after TUI output garbage).
fn strip_ansi_escapes(s: &str) -> String {
    // If no escape sequences, return as-is
    if !s.contains('\x1b') {
        return s.to_string();
    }

    // Find the last escape sequence and take everything after it
    // ANSI escapes are: ESC [ ... (ending with a letter)
    let mut result = s;
    while let Some(esc_pos) = result.rfind('\x1b') {
        // Find the end of this escape sequence (next letter after [)
        let after_esc = &result[esc_pos..];
        if let Some(end_pos) = after_esc.find(|c: char| c.is_ascii_alphabetic()) {
            let clean_part = &result[esc_pos + end_pos + 1..];
            if !clean_part.is_empty() && !clean_part.contains('\x1b') {
                return clean_part.trim().to_string();
            }
        }
        // Try the part before this escape
        result = &result[..esc_pos];
    }

    result.trim().to_string()
}

pub fn read_stdin_paths() -> Vec<PathBuf> {
    let stdin = io::stdin();

    if stdin.is_terminal() {
        return Vec::new();
    }

    let debug = std::env::var("DEBUG").is_ok();

    stdin
        .lock()
        .lines()
        .filter_map(|line| line.ok())
        .enumerate()
        .map(|(i, line)| {
            let cleaned = strip_ansi_escapes(&line);
            let trimmed = cleaned.trim();

            if debug && line != cleaned {
                eprintln!("[DEBUG] Line {} cleaned: {:?} -> {:?}", i, line.len(), trimmed);
            }

            // Strip BOM from first line if present
            if i == 0 {
                trimmed.strip_prefix('\u{feff}').unwrap_or(trimmed).to_string()
            } else {
                trimmed.to_string()
            }
        })
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}
