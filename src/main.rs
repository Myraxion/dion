mod cli;
mod comment;
mod editor;
mod error;
mod help;
mod listing;
mod name;
mod storage;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
