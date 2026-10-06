use crate::{comment, error::Error, storage};
use std::{
    env,
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

pub fn run() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let json = args
        .iter()
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--json");
    match parse_args(&args).and_then(|args| get(&args, json)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            if json {
                let _ = write_json(&mut stderr, &serde_json::json!({ "error": error }));
            } else {
                let _ = write!(stderr, "{}: {}", error.code, error.message);
                if let Some(file) = &error.file {
                    let _ = write!(stderr, " ({})", file.display());
                }
                if let Some(line) = error.line {
                    let _ = write!(stderr, " at line {line}");
                }
                let _ = writeln!(stderr);
            }
            ExitCode::from(error.exit_code)
        }
    }
}

fn parse_args(args: &[OsString]) -> Result<Vec<OsString>, Error> {
    let mut positional = Vec::new();
    let mut after_separator = false;
    let mut json_seen = false;
    for arg in args {
        if after_separator {
            positional.push(arg.clone());
        } else if arg == "--" {
            after_separator = true;
        } else if arg == "--json" && !json_seen {
            json_seen = true;
        } else if arg.to_str().is_some_and(|arg| arg.starts_with('-')) {
            return Err(Error::new(
                "invalid_argument",
                "Unknown or repeated option; use -- before a path starting with -",
                2,
            ));
        } else {
            positional.push(arg.clone());
        }
    }
    Ok(positional)
}

fn get(args: &[OsString], json: bool) -> Result<(), Error> {
    if args.len() != 2 || args[0] != "get" {
        return Err(Error::new(
            "invalid_argument",
            "Usage: dion [--json] get <path> [--json]",
            2,
        ));
    }
    let path = PathBuf::from(&args[1]);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::new("invalid_argument", "Path must have a UTF-8 entry name", 2))?;
    let file = path
        .parent()
        .ok_or_else(|| Error::new("invalid_argument", "Path has no parent", 2))?
        .join("descript.ion");
    let not_found = || Error::new("not_found", "Comment not found", 3).at_file(&file);
    let bytes = storage::read(&file)?.ok_or_else(not_found)?;
    let records = comment::parse(&bytes).map_err(|error| error.at_file(&file))?;
    let key = name.to_lowercase();
    let record = records
        .iter()
        .find(|record| record.name.to_lowercase() == key)
        .ok_or_else(not_found)?;
    let mut output = io::stdout().lock();
    let result = if json {
        write_json(
            &mut output,
            &serde_json::json!({
                "name": record.name, "comment": record.comment, "extension": "none"
            }),
        )
    } else {
        output.write_all(record.comment.as_bytes())
    };
    result.map_err(|error| Error::new("io_error", error.to_string(), 1))
}

fn write_json(output: &mut impl Write, value: &impl serde::Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")
}
