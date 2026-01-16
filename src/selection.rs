use std::collections::HashSet;
use std::path::PathBuf;

pub struct SelectionState {
    selected: HashSet<PathBuf>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            selected: HashSet::new(),
        }
    }

    pub fn add_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        for path in paths {
            if let Ok(canonical) = path.canonicalize() {
                self.selected.insert(canonical);
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

    pub fn is_selected(&self, path: &PathBuf) -> bool {
        path.canonicalize()
            .map(|c| self.selected.contains(&c))
            .unwrap_or(false)
    }

    pub fn count(&self) -> usize {
        self.selected.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.selected.iter()
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
