#![warn(clippy::undocumented_unsafe_blocks)]

mod cli;
mod comment;
mod editor;
mod error;
mod everything;
mod help;
mod i18n;
mod listing;
mod modification;
mod name;
mod storage;
mod terminal;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
