mod cli;
mod comment;
mod editor;
mod error;
mod help;
mod i18n;
mod listing;
mod name;
mod storage;
mod terminal;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
