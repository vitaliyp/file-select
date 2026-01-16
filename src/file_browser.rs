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

    pub fn invalid(path: PathBuf) -> Self {
        let name = extract_name(&path);
        Self {
            path,
            name,
            is_dir: false,
            is_invalid: true,
        }
    }

    fn sort_key(&self) -> (u8, u8, String) {
        // Order: valid dirs, valid files, invalid entries
        // Within each group: alphabetically by lowercase name
        let invalid_order = if self.is_invalid { 1 } else { 0 };
        let dir_order = if self.is_dir { 0 } else { 1 };
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
    pub show_hidden: bool,
    invalid_paths: Vec<PathBuf>,
}

impl BrowserState {
    pub fn new(start_dir: PathBuf, show_hidden: bool) -> Result<Self> {
        let current_dir = start_dir.canonicalize()?;
        let mut state = Self {
            current_dir,
            entries: Vec::new(),
            cursor: 0,
            show_hidden,
            invalid_paths: Vec::new(),
        };
        state.refresh()?;
        Ok(state)
    }

    pub fn set_invalid_paths(&mut self, paths: Vec<PathBuf>) {
        self.invalid_paths = paths;
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
        for invalid_path in &self.invalid_paths {
            if self.should_show_invalid_entry(invalid_path) {
                self.entries.push(FileEntry::invalid(invalid_path.clone()));
            }
        }
    }

    fn should_show_invalid_entry(&self, path: &Path) -> bool {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.current_dir.join(path)
        };

        let Some(parent) = full_path.parent() else {
            return false;
        };

        let parent_matches = parent == self.current_dir
            || parent.canonicalize().ok().as_ref() == Some(&self.current_dir);

        if !parent_matches {
            return false;
        }

        let name = extract_name(&full_path);
        !self.entries.iter().any(|e| e.name == name)
    }

    fn clamp_cursor(&mut self) {
        if self.cursor >= self.entries.len() {
            self.cursor = self.entries.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
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
