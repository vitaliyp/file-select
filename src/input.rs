use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;

pub fn read_stdin_paths() -> Vec<PathBuf> {
    let stdin = io::stdin();

    if stdin.is_terminal() {
        return Vec::new();
    }

    stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}
