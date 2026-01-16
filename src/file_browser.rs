use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

impl FileEntry {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let is_dir = path.is_dir();
        Self { path, name, is_dir }
    }
}

pub struct BrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub cursor: usize,
    pub show_hidden: bool,
}

impl BrowserState {
    pub fn new(start_dir: PathBuf, show_hidden: bool) -> color_eyre::Result<Self> {
        let current_dir = start_dir.canonicalize()?;
        let mut state = Self {
            current_dir,
            entries: Vec::new(),
            cursor: 0,
            show_hidden,
        };
        state.refresh()?;
        Ok(state)
    }

    pub fn refresh(&mut self) -> color_eyre::Result<()> {
        self.entries = read_directory(&self.current_dir, self.show_hidden)?;
        if self.cursor >= self.entries.len() {
            self.cursor = self.entries.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
        }
    }

    pub fn enter_directory(&mut self) -> color_eyre::Result<bool> {
        if let Some(entry) = self.entries.get(self.cursor) {
            if entry.is_dir {
                self.current_dir = entry.path.clone();
                self.cursor = 0;
                self.refresh()?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn go_parent(&mut self) -> color_eyre::Result<bool> {
        if let Some(parent) = self.current_dir.parent() {
            let old_dir = self.current_dir.clone();
            self.current_dir = parent.to_path_buf();
            self.refresh()?;
            // Try to position cursor on the directory we came from
            if let Some(pos) = self.entries.iter().position(|e| e.path == old_dir) {
                self.cursor = pos;
            } else {
                self.cursor = 0;
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub fn toggle_hidden(&mut self) -> color_eyre::Result<()> {
        self.show_hidden = !self.show_hidden;
        self.refresh()
    }

    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }
}

fn read_directory(path: &PathBuf, show_hidden: bool) -> color_eyre::Result<Vec<FileEntry>> {
    let mut entries: Vec<FileEntry> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| FileEntry::new(e.path()))
        .filter(|e| show_hidden || !e.name.starts_with('.'))
        .collect();

    // Sort: directories first, then alphabetically by name (case-insensitive)
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
