use crossterm::event::{KeyCode, KeyEvent};
use std::fs;
use std::path::PathBuf;

use crate::file_browser::BrowserState;
use crate::selection::SelectionState;

pub enum AppAction {
    Continue,
    Quit,
    Confirm,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FocusedPane {
    Files,
    Selected,
}

pub struct App {
    pub browser: BrowserState,
    pub selection: SelectionState,
    pub use_absolute: bool,
    pub base_dir: PathBuf,
    pub focused_pane: FocusedPane,
    pub selected_cursor: usize,
}

impl App {
    pub fn new(
        start_dir: PathBuf,
        show_hidden: bool,
        use_absolute: bool,
        pre_selected: Vec<PathBuf>,
    ) -> color_eyre::Result<Self> {
        let base_dir = start_dir.canonicalize()?;
        let mut browser = BrowserState::new(start_dir, show_hidden)?;
        let mut selection = SelectionState::new();
        selection.add_paths(pre_selected);

        // Sync invalid paths to browser
        let invalid_paths: Vec<PathBuf> = selection.iter_invalid().cloned().collect();
        browser.set_invalid_paths(invalid_paths);
        browser.refresh()?;

        Ok(Self {
            browser,
            selection,
            use_absolute,
            base_dir,
            focused_pane: FocusedPane::Files,
            selected_cursor: 0,
        })
    }

    /// Get sorted list of selected paths for display
    pub fn get_selected_list(&self) -> Vec<(PathBuf, bool)> {
        let mut items: Vec<(PathBuf, bool)> = self
            .selection
            .iter_valid()
            .map(|p| (p.clone(), true))
            .chain(self.selection.iter_invalid().map(|p| (p.clone(), false)))
            .collect();
        items.sort_by(|a, b| {
            let a_display = self.path_display(&a.0, a.1);
            let b_display = self.path_display(&b.0, b.1);
            a_display.cmp(&b_display)
        });
        items
    }

    fn path_display(&self, path: &PathBuf, is_valid: bool) -> String {
        if is_valid {
            path.strip_prefix(&self.base_dir)
                .map(|rel| format!("./{}", rel.display()))
                .unwrap_or_else(|_| path.display().to_string())
        } else {
            let s = path.to_string_lossy();
            if s.starts_with("./") || s.starts_with('/') {
                s.to_string()
            } else {
                format!("./{}", s)
            }
        }
    }

    fn clamp_selected_cursor(&mut self) {
        let count = self.selection.count();
        if count == 0 {
            self.selected_cursor = 0;
        } else if self.selected_cursor >= count {
            self.selected_cursor = count - 1;
        }
    }

    fn sync_invalid_paths(&mut self) -> color_eyre::Result<()> {
        let invalid_paths: Vec<PathBuf> = self.selection.iter_invalid().cloned().collect();
        self.browser.set_invalid_paths(invalid_paths);
        self.browser.refresh()
    }

    /// Recursively collect all files in a directory
    fn collect_files_recursive(&self, dir: &PathBuf) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    // Recurse into subdirectory
                    files.extend(self.collect_files_recursive(&path));
                } else {
                    // Check hidden file filter
                    let name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if self.browser.show_hidden || !name.starts_with('.') {
                        files.push(path);
                    }
                }
            }
        }
        files
    }

    /// Toggle all files recursively in the current entry (if it's a directory)
    fn toggle_recursive(&mut self) {
        if let Some(entry) = self.browser.current_entry().cloned() {
            if entry.is_dir && !entry.is_invalid {
                let files = self.collect_files_recursive(&entry.path);
                if files.is_empty() {
                    return;
                }
                // Check if all files are already selected
                let all_selected = files.iter().all(|f| self.selection.is_selected(f));
                if all_selected {
                    self.selection.remove_paths(&files);
                } else {
                    self.selection.add_paths(files);
                }
            }
        }
    }

    /// Toggle all entries in the current directory
    fn toggle_all_in_current(&mut self) {
        let paths: Vec<PathBuf> = self.browser.entries
            .iter()
            .filter(|e| !e.is_invalid)
            .map(|e| e.path.clone())
            .collect();
        if paths.is_empty() {
            return;
        }
        // Check if all are already selected
        let all_selected = paths.iter().all(|p| self.selection.is_selected(p));
        if all_selected {
            self.selection.remove_paths(&paths);
        } else {
            self.selection.add_paths(paths);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> color_eyre::Result<AppAction> {
        match key.code {
            // Quit without output
            KeyCode::Char('q') | KeyCode::Esc => Ok(AppAction::Quit),

            // Confirm and exit with output
            KeyCode::Enter => Ok(AppAction::Confirm),

            // Switch panes
            KeyCode::Tab => {
                self.focused_pane = match self.focused_pane {
                    FocusedPane::Files => FocusedPane::Selected,
                    FocusedPane::Selected => FocusedPane::Files,
                };
                self.clamp_selected_cursor();
                Ok(AppAction::Continue)
            }

            // Move up
            KeyCode::Char('k') | KeyCode::Up => {
                match self.focused_pane {
                    FocusedPane::Files => self.browser.move_up(),
                    FocusedPane::Selected => {
                        if self.selected_cursor > 0 {
                            self.selected_cursor -= 1;
                        }
                    }
                }
                Ok(AppAction::Continue)
            }

            // Move down
            KeyCode::Char('j') | KeyCode::Down => {
                match self.focused_pane {
                    FocusedPane::Files => self.browser.move_down(),
                    FocusedPane::Selected => {
                        let count = self.selection.count();
                        if count > 0 && self.selected_cursor + 1 < count {
                            self.selected_cursor += 1;
                        }
                    }
                }
                Ok(AppAction::Continue)
            }

            // Go to parent directory (files pane only)
            KeyCode::Char('h') | KeyCode::Left => {
                if self.focused_pane == FocusedPane::Files {
                    let _ = self.browser.go_parent();
                }
                Ok(AppAction::Continue)
            }

            // Enter directory (files pane only)
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focused_pane == FocusedPane::Files {
                    let _ = self.browser.enter_directory();
                }
                Ok(AppAction::Continue)
            }

            // Toggle selection / Deselect in selected pane
            KeyCode::Char(' ') => {
                match self.focused_pane {
                    FocusedPane::Files => {
                        if let Some(entry) = self.browser.current_entry().cloned() {
                            if entry.is_invalid {
                                self.selection.toggle_invalid(&entry.path);
                                self.sync_invalid_paths()?;
                            } else {
                                self.selection.toggle(&entry.path);
                            }
                        }
                    }
                    FocusedPane::Selected => {
                        let items = self.get_selected_list();
                        if let Some((path, is_valid)) = items.get(self.selected_cursor).cloned() {
                            if is_valid {
                                self.selection.remove_paths(&[path]);
                            } else {
                                self.selection.toggle_invalid(&path);
                                self.sync_invalid_paths()?;
                            }
                            self.clamp_selected_cursor();
                        }
                    }
                }
                Ok(AppAction::Continue)
            }

            // Toggle recursive select (for directories, files pane only)
            KeyCode::Char('r') => {
                if self.focused_pane == FocusedPane::Files {
                    self.toggle_recursive();
                }
                Ok(AppAction::Continue)
            }

            // Toggle all in current directory (files pane only)
            KeyCode::Char('a') => {
                if self.focused_pane == FocusedPane::Files {
                    self.toggle_all_in_current();
                }
                Ok(AppAction::Continue)
            }

            // Toggle hidden files
            KeyCode::Char('.') => {
                self.browser.toggle_hidden()?;
                Ok(AppAction::Continue)
            }

            _ => Ok(AppAction::Continue),
        }
    }

    pub fn get_output(&self) -> Vec<String> {
        self.selection.to_output(self.use_absolute, &self.base_dir)
    }
}
