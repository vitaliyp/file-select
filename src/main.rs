mod app;
mod config;
mod file_browser;
mod input;
mod selection;
mod ui;

use std::fs::{self, File};
use std::io::{self, BufRead, IsTerminal, Write};
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

    // Read from selections file if specified
    let file_paths: Vec<PathBuf> = if let Some(ref file_path) = config.selections_file {
        read_selections_file(file_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Combine pre-selected paths from CLI args, stdin, and file
    let mut pre_selected: Vec<PathBuf> = config.files.clone();
    pre_selected.extend(stdin_paths);
    pre_selected.extend(file_paths);

    // Get starting directory
    let start_dir = std::env::current_dir()?;

    // Create app state
    let mut app = App::new(
        start_dir,
        config.show_hidden,
        config.use_absolute_paths(),
        pre_selected,
    )?;

    // Open /dev/tty for TUI output and keyboard input
    // This keeps stdout clean for piping selected paths
    let mut tty = File::options().read(true).write(true).open("/dev/tty")?;

    // Redirect stdin to /dev/tty for keyboard input if it was piped
    if !io::stdin().is_terminal() {
        unsafe {
            libc::dup2(tty.as_raw_fd(), 0);
        }
    }

    // Setup terminal - write TUI to /dev/tty, not stdout
    enable_raw_mode()?;
    execute!(tty, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(tty);
    let mut terminal = Terminal::new(backend)?;

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
            let output = app.get_output();
            if let Some(ref file_path) = config.selections_file {
                // Write to selections file
                write_selections_file(file_path, &output)?;
            } else {
                // Output to stdout
                for path in output {
                    println!("{}", path);
                }
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

fn read_selections_file(path: &PathBuf) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let paths = reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();
    Ok(paths)
}

fn write_selections_file(path: &PathBuf, paths: &[String]) -> Result<()> {
    let mut file = fs::File::create(path)?;
    for p in paths {
        writeln!(file, "{}", p)?;
    }
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<File>>,
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
