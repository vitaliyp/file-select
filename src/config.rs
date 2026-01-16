use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "file-list")]
#[command(about = "TUI file selector with vim-style navigation")]
pub struct Config {
    /// Output absolute paths instead of relative
    #[arg(short = 'a', long = "absolute")]
    pub absolute: bool,

    /// Output relative paths (default)
    #[arg(short = 'r', long = "relative")]
    pub relative: bool,

    /// Show hidden files by default
    #[arg(short = 'H', long = "hidden")]
    pub show_hidden: bool,

    /// Pre-selected files
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,
}

impl Config {
    pub fn use_absolute_paths(&self) -> bool {
        self.absolute && !self.relative
    }
}
