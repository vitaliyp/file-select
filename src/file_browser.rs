use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Result;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_invalid: bool,
}

impl FileEntry {
    pub fn from_path(path: PathBuf) -> Self {
        let name = extract_name(&path);
        let is_dir = path.is_dir();
        Self {
            path,
            name,
            is_dir,
            is_invalid: false,
        }
    }

    pub fn invalid(path: PathBuf, display_name: String) -> Self {
        Self {
            path,
            name: display_name,
            is_dir: false,
            is_invalid: true,
        }
    }

    fn sort_key(&self) -> (u8, u8, String) {
        let invalid_order = u8::from(self.is_invalid);
        let dir_order = u8::from(!self.is_dir);
        (invalid_order, dir_order, self.name.to_lowercase())
    }
}

fn extract_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[derive(Debug)]
pub struct BrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub show_hidden: bool,
    base_dir: PathBuf,
    invalid_paths: Vec<PathBuf>,
}

impl BrowserState {
    pub fn new(start_dir: PathBuf, show_hidden: bool) -> Result<Self> {
        let current_dir = start_dir.canonicalize()?;
        let mut state = Self {
            base_dir: current_dir.clone(),
            current_dir,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            show_hidden,
            invalid_paths: Vec::new(),
        };
        state.refresh()?;
        Ok(state)
    }

    pub fn add_invalid_paths(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if !self.invalid_paths.contains(&path) {
                self.invalid_paths.push(path);
            }
        }
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.entries = self.read_current_directory()?;
        self.add_invalid_entries();
        self.entries.sort_by_key(|e| e.sort_key());
        self.clamp_cursor();
        Ok(())
    }

    fn read_current_directory(&self) -> Result<Vec<FileEntry>> {
        let entries = fs::read_dir(&self.current_dir)?
            .filter_map(|e| e.ok())
            .map(|e| FileEntry::from_path(e.path()))
            .filter(|e| self.show_hidden || !e.name.starts_with('.'))
            .collect();
        Ok(entries)
    }

    fn add_invalid_entries(&mut self) {
        let entries_to_add: Vec<_> = self
            .invalid_paths
            .iter()
            .filter_map(|path| self.resolve_invalid_for_current_dir(path))
            .collect();

        for (path, display_name) in entries_to_add {
            if !self.entries.iter().any(|e| e.name == display_name) {
                self.entries.push(FileEntry::invalid(path, display_name));
            }
        }
    }

    /// For an invalid path, determine if it should be shown in current_dir.
    /// Returns Some((original_path, display_name)) if it should be shown here.
    fn resolve_invalid_for_current_dir(&self, path: &Path) -> Option<(PathBuf, String)> {
        let (display_dir, display_name) = self.find_display_location(path)?;

        if display_dir == self.current_dir {
            Some((path.to_path_buf(), display_name))
        } else {
            None
        }
    }

    /// Find where an invalid path should be displayed.
    /// Returns (directory_to_show_in, name_to_display).
    fn find_display_location(&self, path: &Path) -> Option<(PathBuf, String)> {
        // Make path absolute relative to base_dir
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        };

        // Get path relative to base_dir
        let relative = full_path.strip_prefix(&self.base_dir).ok()?;
        let components: Vec<_> = relative.components().collect();

        if components.is_empty() {
            return None;
        }

        // Walk from base_dir, find where the path becomes invalid
        let mut current = self.base_dir.clone();

        for (i, component) in components.iter().enumerate() {
            let next = current.join(component);

            if !next.exists() {
                // This component doesn't exist
                // current is the display directory
                // remaining components form the display name
                let remaining: PathBuf = components[i..].iter().collect();
                let display_name = remaining.to_string_lossy().into_owned();
                let display_dir = current.canonicalize().unwrap_or(current);
                return Some((display_dir, display_name));
            }

            current = next;
        }

        // Path actually exists - shouldn't happen for invalid paths
        None
    }

    fn clamp_cursor(&mut self) {
        if self.cursor >= self.entries.len() {
            self.cursor = self.entries.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            // When moving up, keep cursor at top of visible area
            self.scroll_offset = self.scroll_offset.min(self.cursor);
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
        }
    }

    /// Adjust scroll offset to keep cursor visible. Call this during render
    /// when visible_height is known.
    pub fn adjust_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        // Ensure cursor is visible at bottom when scrolling down
        if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    pub fn enter_directory(&mut self) -> Result<bool> {
        let Some(entry) = self.entries.get(self.cursor) else {
            return Ok(false);
        };

        if !entry.is_dir {
            return Ok(false);
        }

        self.current_dir = entry.path.clone();
        self.cursor = 0;
        self.scroll_offset = 0;
        self.refresh()?;
        Ok(true)
    }

    pub fn go_parent(&mut self) -> Result<bool> {
        let Some(parent) = self.current_dir.parent() else {
            return Ok(false);
        };

        let old_dir = self.current_dir.clone();
        self.current_dir = parent.to_path_buf();
        self.refresh()?;

        self.cursor = self
            .entries
            .iter()
            .position(|e| e.path == old_dir)
            .unwrap_or(0);
        self.scroll_offset = self.cursor; // Position cursor at top

        Ok(true)
    }

    pub fn toggle_hidden(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.refresh()
    }

    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }
}
