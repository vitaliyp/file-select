mod app;
mod config;
mod file_browser;
mod input;
mod selection;
mod ui;

use std::fs::File;
use std::io::{self, IsTerminal, Stdout};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use app::{App, AppAction};
use config::Config;

fn main() -> Result<()> {
    color_eyre::install()?;

    // Read stdin paths before we switch to TUI mode
    let stdin_paths = input::read_stdin_paths();

    // Parse CLI arguments
    let config = Config::parse();

    // Combine pre-selected paths from CLI args and stdin
    let mut pre_selected: Vec<PathBuf> = config.files.clone();
    pre_selected.extend(stdin_paths);

    // Get starting directory
    let start_dir = std::env::current_dir()?;

    // Create app state
    let mut app = App::new(
        start_dir,
        config.show_hidden,
        config.use_absolute_paths(),
        pre_selected,
    )?;

    // If stdin was piped, we need to reopen /dev/tty for keyboard input
    let tty_fd = if !io::stdin().is_terminal() {
        let tty = File::open("/dev/tty")?;
        Some(tty)
    } else {
        None
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // If we opened /dev/tty, redirect stdin to it
    if let Some(tty) = tty_fd {
        unsafe {
            libc::dup2(tty.as_raw_fd(), 0);
        }
    }

    // Run the main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle result
    match result {
        Ok(true) => {
            // User confirmed - output selected paths
            for path in app.get_output() {
                println!("{}", path);
            }
        }
        Ok(false) => {
            // User quit without confirming
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<bool> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.handle_key(key)? {
                AppAction::Continue => {}
                AppAction::Quit => return Ok(false),
                AppAction::Confirm => return Ok(true),
            }
        }
    }
}
