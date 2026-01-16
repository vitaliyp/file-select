use crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;

use crate::file_browser::BrowserState;
use crate::selection::SelectionState;

pub enum AppAction {
    Continue,
    Quit,
    Confirm,
}

pub struct App {
    pub browser: BrowserState,
    pub selection: SelectionState,
    pub use_absolute: bool,
    pub base_dir: PathBuf,
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
        })
    }

    fn sync_invalid_paths(&mut self) -> color_eyre::Result<()> {
        let invalid_paths: Vec<PathBuf> = self.selection.iter_invalid().cloned().collect();
        self.browser.set_invalid_paths(invalid_paths);
        self.browser.refresh()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> color_eyre::Result<AppAction> {
        match key.code {
            // Quit without output
            KeyCode::Char('q') | KeyCode::Esc => Ok(AppAction::Quit),

            // Confirm and exit with output
            KeyCode::Enter => Ok(AppAction::Confirm),

            // Move up
            KeyCode::Char('k') | KeyCode::Up => {
                self.browser.move_up();
                Ok(AppAction::Continue)
            }

            // Move down
            KeyCode::Char('j') | KeyCode::Down => {
                self.browser.move_down();
                Ok(AppAction::Continue)
            }

            // Go to parent directory
            KeyCode::Char('h') | KeyCode::Left => {
                let _ = self.browser.go_parent();
                Ok(AppAction::Continue)
            }

            // Enter directory
            KeyCode::Char('l') | KeyCode::Right => {
                let _ = self.browser.enter_directory();
                Ok(AppAction::Continue)
            }

            // Toggle selection
            KeyCode::Char(' ') => {
                if let Some(entry) = self.browser.current_entry().cloned() {
                    if entry.is_invalid {
                        self.selection.toggle_invalid(&entry.path);
                        self.sync_invalid_paths()?;
                    } else {
                        self.selection.toggle(&entry.path);
                    }
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
