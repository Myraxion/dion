mod cli;
mod comment;
mod error;
mod storage;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
