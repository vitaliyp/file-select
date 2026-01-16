use std::path::PathBuf;

use clap::Parser;

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

    /// Selections file to read from and write to
    #[arg(short = 'f', long = "file")]
    pub selections_file: Option<PathBuf>,

    /// Pre-selected files
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,
}

impl Config {
    pub fn use_absolute_paths(&self) -> bool {
        self.absolute && !self.relative
    }
}
