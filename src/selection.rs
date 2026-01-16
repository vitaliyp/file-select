use std::collections::HashSet;
use std::path::PathBuf;

pub struct SelectionState {
    /// Valid paths (canonicalized, files exist)
    selected: HashSet<PathBuf>,
    /// Invalid paths (files don't exist, stored as provided)
    invalid: HashSet<PathBuf>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            selected: HashSet::new(),
            invalid: HashSet::new(),
        }
    }

    pub fn add_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        for path in paths {
            if let Ok(canonical) = path.canonicalize() {
                self.selected.insert(canonical);
            } else {
                // Store invalid paths as-is
                self.invalid.insert(path);
            }
        }
    }

    pub fn remove_paths(&mut self, paths: &[PathBuf]) {
        for path in paths {
            if let Ok(canonical) = path.canonicalize() {
                self.selected.remove(&canonical);
            }
        }
    }

    pub fn toggle(&mut self, path: &PathBuf) {
        if let Ok(canonical) = path.canonicalize() {
            if self.selected.contains(&canonical) {
                self.selected.remove(&canonical);
            } else {
                self.selected.insert(canonical);
            }
        }
    }

    /// Toggle an invalid (non-existent) path
    pub fn toggle_invalid(&mut self, path: &PathBuf) {
        if self.invalid.contains(path) {
            self.invalid.remove(path);
        } else {
            self.invalid.insert(path.clone());
        }
    }

    pub fn is_selected(&self, path: &PathBuf) -> bool {
        path.canonicalize()
            .map(|c| self.selected.contains(&c))
            .unwrap_or(false)
    }

    pub fn is_invalid_selected(&self, path: &PathBuf) -> bool {
        self.invalid.contains(path)
    }

    pub fn count(&self) -> usize {
        self.selected.len() + self.invalid.len()
    }

    pub fn iter_valid(&self) -> impl Iterator<Item = &PathBuf> {
        self.selected.iter()
    }

    pub fn iter_invalid(&self) -> impl Iterator<Item = &PathBuf> {
        self.invalid.iter()
    }

    pub fn to_output(&self, use_absolute: bool, base_dir: &PathBuf) -> Vec<String> {
        let mut paths: Vec<String> = self
            .selected
            .iter()
            .map(|p| {
                if use_absolute {
                    p.to_string_lossy().to_string()
                } else {
                    p.strip_prefix(base_dir)
                        .map(|rel| format!("./{}", rel.to_string_lossy()))
                        .unwrap_or_else(|_| p.to_string_lossy().to_string())
                }
            })
            .chain(self.invalid.iter().map(|p| {
                if use_absolute {
                    // Try to make it absolute relative to base_dir
                    base_dir
                        .join(p)
                        .to_string_lossy()
                        .to_string()
                } else {
                    let s = p.to_string_lossy();
                    if s.starts_with("./") || s.starts_with('/') {
                        s.to_string()
                    } else {
                        format!("./{}", s)
                    }
                }
            }))
            .collect();
        paths.sort();
        paths
    }

}

impl Default for SelectionState {
    fn default() -> Self {
        Self::new()
    }
}
