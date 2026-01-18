mod app;
mod config;
mod file_browser;
mod input;
mod selection;
mod ui;

use std::fs::File;
use std::io::{self, BufRead, IsTerminal, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;

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

    let stdin_paths = input::read_stdin_paths();
    let config = Config::parse();

    let file_paths = config
        .selections_file
        .as_ref()
        .map(|p| read_selections_file(p))
        .transpose()?
        .unwrap_or_default();

    let pre_selected = [config.files.clone(), stdin_paths, file_paths].concat();
    let start_dir = std::env::current_dir()?;

    let mut app = App::new(
        start_dir,
        config.show_hidden,
        config.use_absolute_paths(),
        pre_selected,
        config.selections_file.clone(),
    )?;

    let confirmed = run_tui(&mut app)?;

    if confirmed {
        let output = app.get_output();
        if let Some(ref path) = config.selections_file {
            write_selections_file(path, &output)?;
        } else {
            for path in output {
                println!("{}", path);
            }
        }
    }

    Ok(())
}

fn run_tui(app: &mut App) -> Result<bool> {
    let mut tty = File::options().read(true).write(true).open("/dev/tty")?;

    if !io::stdin().is_terminal() {
        unsafe {
            libc::dup2(tty.as_raw_fd(), 0);
        }
    }

    enable_raw_mode()?;
    execute!(tty, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(tty);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<File>>, app: &mut App) -> Result<bool> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.handle_key(key)? {
                AppAction::Continue => {}
                AppAction::Quit => return Ok(false),
                AppAction::Confirm => return Ok(true),
                AppAction::Save => {
                    if let Some(path) = app.selections_file() {
                        let output = app.get_output();
                        write_selections_file(path, &output)?;
                    }
                }
            }
        }
    }
}

fn read_selections_file(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let paths = io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .map(std::path::PathBuf::from)
        .collect();

    Ok(paths)
}

fn write_selections_file(path: &Path, paths: &[String]) -> Result<()> {
    let mut file = File::create(path)?;
    for p in paths {
        writeln!(file, "{}", p)?;
    }
    Ok(())
}
