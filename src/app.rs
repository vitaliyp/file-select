use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::file_browser::BrowserState;
use crate::selection::SelectionState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    #[default]
    Files,
    Selected,
}

impl FocusedPane {
    fn toggle(self) -> Self {
        match self {
            Self::Files => Self::Selected,
            Self::Selected => Self::Files,
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub browser: BrowserState,
    pub selection: SelectionState,
    pub base_dir: PathBuf,
    pub focused_pane: FocusedPane,
    pub selected_cursor: usize,
    pub selected_scroll_offset: usize,
    use_absolute: bool,
}

impl App {
    pub fn new(
        start_dir: PathBuf,
        show_hidden: bool,
        use_absolute: bool,
        pre_selected: Vec<PathBuf>,
    ) -> Result<Self> {
        let base_dir = start_dir.canonicalize()?;
        let mut browser = BrowserState::new(start_dir, show_hidden)?;
        let mut selection = SelectionState::new();
        selection.add_paths(pre_selected);

        let invalid_paths: Vec<PathBuf> = selection.iter_invalid().cloned().collect();
        browser.add_invalid_paths(invalid_paths);
        browser.refresh()?;

        Ok(Self {
            browser,
            selection,
            use_absolute,
            base_dir,
            focused_pane: FocusedPane::default(),
            selected_cursor: 0,
            selected_scroll_offset: 0,
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<AppAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(AppAction::Quit),
            KeyCode::Enter => Ok(AppAction::Confirm),
            KeyCode::Tab => {
                self.focused_pane = self.focused_pane.toggle();
                self.clamp_selected_cursor();
                Ok(AppAction::Continue)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up();
                Ok(AppAction::Continue)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down();
                Ok(AppAction::Continue)
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.focused_pane == FocusedPane::Files {
                    let _ = self.browser.go_parent();
                }
                Ok(AppAction::Continue)
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focused_pane == FocusedPane::Files {
                    let _ = self.browser.enter_directory();
                }
                Ok(AppAction::Continue)
            }
            KeyCode::Char(' ') => {
                self.handle_space();
                Ok(AppAction::Continue)
            }
            KeyCode::Char('r') => {
                if self.focused_pane == FocusedPane::Files {
                    self.toggle_recursive();
                }
                Ok(AppAction::Continue)
            }
            KeyCode::Char('a') => {
                if self.focused_pane == FocusedPane::Files {
                    self.toggle_all_in_current();
                }
                Ok(AppAction::Continue)
            }
            KeyCode::Char('.') => {
                self.browser.toggle_hidden()?;
                Ok(AppAction::Continue)
            }
            _ => Ok(AppAction::Continue),
        }
    }

    fn move_up(&mut self) {
        match self.focused_pane {
            FocusedPane::Files => self.browser.move_up(),
            FocusedPane::Selected => {
                if self.selected_cursor > 0 {
                    self.selected_cursor -= 1;
                    // When moving up, keep cursor at top of visible area
                    self.selected_scroll_offset = self.selected_scroll_offset.min(self.selected_cursor);
                }
            }
        }
    }

    fn move_down(&mut self) {
        match self.focused_pane {
            FocusedPane::Files => self.browser.move_down(),
            FocusedPane::Selected => {
                let count = self.selection.count();
                if count > 0 && self.selected_cursor + 1 < count {
                    self.selected_cursor += 1;
                }
            }
        }
    }

    pub fn adjust_selected_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        // Ensure cursor is visible at bottom when scrolling down
        if self.selected_cursor >= self.selected_scroll_offset + visible_height {
            self.selected_scroll_offset = self.selected_cursor - visible_height + 1;
        }
    }

    fn handle_space(&mut self) {
        match self.focused_pane {
            FocusedPane::Files => self.toggle_current_entry(),
            FocusedPane::Selected => self.deselect_at_cursor(),
        }
    }

    fn toggle_current_entry(&mut self) {
        let Some(entry) = self.browser.current_entry().cloned() else {
            return;
        };

        if entry.is_invalid {
            // Invalid file is already in browser, just toggle selection state
            self.selection.toggle_invalid(&entry.path);
        } else {
            self.selection.toggle(&entry.path);
        }
    }

    fn deselect_at_cursor(&mut self) {
        let items = self.get_selected_list();
        let Some((path, is_valid)) = items.get(self.selected_cursor).cloned() else {
            return;
        };

        if is_valid {
            self.selection.remove_paths(&[path]);
        } else {
            // Invalid file stays in browser, just deselect it
            self.selection.toggle_invalid(&path);
        }
        self.clamp_selected_cursor();
    }

    fn toggle_recursive(&mut self) {
        let Some(entry) = self.browser.current_entry().cloned() else {
            return;
        };

        if !entry.is_dir || entry.is_invalid {
            return;
        }

        let files = self.collect_files_recursive(&entry.path);
        if files.is_empty() {
            return;
        }

        let all_selected = files.iter().all(|f| self.selection.is_selected(f));
        if all_selected {
            self.selection.remove_paths(&files);
        } else {
            self.selection.add_paths(files);
        }
    }

    fn toggle_all_in_current(&mut self) {
        let paths: Vec<PathBuf> = self
            .browser
            .entries
            .iter()
            .filter(|e| !e.is_invalid)
            .map(|e| e.path.clone())
            .collect();

        if paths.is_empty() {
            return;
        }

        let all_selected = paths.iter().all(|p| self.selection.is_selected(p));
        if all_selected {
            self.selection.remove_paths(&paths);
        } else {
            self.selection.add_paths(paths);
        }
    }

    fn collect_files_recursive(&self, dir: &Path) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };

        entries
            .filter_map(|e| e.ok())
            .flat_map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    self.collect_files_recursive(&path)
                } else {
                    let dominated_by_hidden = path
                        .file_name()
                        .map(|n| n.to_string_lossy().starts_with('.'))
                        .unwrap_or(false);

                    if self.browser.show_hidden || !dominated_by_hidden {
                        vec![path]
                    } else {
                        vec![]
                    }
                }
            })
            .collect()
    }

    fn clamp_selected_cursor(&mut self) {
        let count = self.selection.count();
        if count == 0 {
            self.selected_cursor = 0;
        } else if self.selected_cursor >= count {
            self.selected_cursor = count - 1;
        }
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
            let a_display = self.format_path_for_display(&a.0, a.1);
            let b_display = self.format_path_for_display(&b.0, b.1);
            a_display.cmp(&b_display)
        });
        items
    }

    pub fn format_path_for_display(&self, path: &Path, is_valid: bool) -> String {
        if is_valid {
            path.strip_prefix(&self.base_dir)
                .map(|rel| format!("./{}", rel.display()))
                .unwrap_or_else(|_| path.display().to_string())
        } else {
            let s = path.to_string_lossy();
            if s.starts_with("./") || s.starts_with('/') {
                s.into_owned()
            } else {
                format!("./{}", s)
            }
        }
    }

    pub fn get_output(&self) -> Vec<String> {
        self.selection.to_output(self.use_absolute, &self.base_dir)
    }
}
