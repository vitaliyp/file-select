use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct SelectionState {
    /// Valid paths (canonicalized, files exist)
    valid: HashSet<PathBuf>,
    /// Invalid paths (files don't exist, stored as provided)
    invalid: HashSet<PathBuf>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        for path in paths {
            match path.canonicalize() {
                Ok(canonical) => {
                    self.valid.insert(canonical);
                }
                Err(_) => {
                    self.invalid.insert(path);
                }
            }
        }
    }

    pub fn remove_paths(&mut self, paths: &[PathBuf]) {
        for path in paths {
            if let Ok(canonical) = path.canonicalize() {
                self.valid.remove(&canonical);
            }
        }
    }

    pub fn toggle(&mut self, path: &Path) {
        if let Ok(canonical) = path.canonicalize() {
            if !self.valid.remove(&canonical) {
                self.valid.insert(canonical);
            }
        }
    }

    pub fn toggle_invalid(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if !self.invalid.remove(&path) {
            self.invalid.insert(path);
        }
    }

    pub fn is_selected(&self, path: &Path) -> bool {
        path.canonicalize()
            .map(|c| self.valid.contains(&c))
            .unwrap_or(false)
    }

    pub fn is_invalid_selected(&self, path: &Path) -> bool {
        self.invalid.contains(path)
    }

    pub fn count(&self) -> usize {
        self.valid.len() + self.invalid.len()
    }

    pub fn iter_valid(&self) -> impl Iterator<Item = &PathBuf> {
        self.valid.iter()
    }

    pub fn iter_invalid(&self) -> impl Iterator<Item = &PathBuf> {
        self.invalid.iter()
    }

    pub fn to_output(&self, use_absolute: bool, base_dir: &Path) -> Vec<String> {
        let mut paths: Vec<String> = self
            .valid
            .iter()
            .map(|p| format_path(p, base_dir, use_absolute))
            .chain(
                self.invalid
                    .iter()
                    .map(|p| format_invalid_path(p, base_dir, use_absolute)),
            )
            .collect();
        paths.sort();
        paths
    }
}

fn format_path(path: &Path, base_dir: &Path, use_absolute: bool) -> String {
    if use_absolute {
        path.to_string_lossy().into_owned()
    } else {
        path.strip_prefix(base_dir)
            .map(|rel| format!("./{}", rel.display()))
            .unwrap_or_else(|_| path.to_string_lossy().into_owned())
    }
}

fn format_invalid_path(path: &Path, base_dir: &Path, use_absolute: bool) -> String {
    if use_absolute {
        base_dir.join(path).to_string_lossy().into_owned()
    } else {
        let s = path.to_string_lossy();
        if s.starts_with("./") || s.starts_with('/') {
            s.into_owned()
        } else {
            format!("./{}", s)
        }
    }
}
