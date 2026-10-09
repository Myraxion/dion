use crate::error::Error;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Keeps the text until the caller explicitly removes it after a successful edit.
pub fn create(body: &str) -> Result<PathBuf, Error> {
    let temporary = tempfile::Builder::new()
        .prefix("dion-edit-")
        .suffix(".txt")
        .tempfile()
        .map_err(|error| Error::new("io_error", error.to_string(), 1))?;
    let (mut output, path) = temporary
        .keep()
        .map_err(|error| Error::new("io_error", error.to_string(), 1))?;
    output
        .write_all(body.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|error| recovery(Error::io(error, &path), &path))?;
    Ok(path)
}

pub fn recovery(mut error: Error, text: &Path) -> Error {
    error
        .message
        .push_str(&format!("; Edit text retained at {}", text.display()));
    error
}

#[cfg(windows)]
pub fn run(text: &Path) -> Result<(), Error> {
    use std::{
        env,
        ffi::OsString,
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            process::CommandExt,
        },
        process::{Command, Stdio},
    };
    let configuration = ["VISUAL", "EDITOR"]
        .iter()
        .find_map(|key| env::var_os(key).filter(|value| !value.is_empty()))
        .unwrap_or_else(|| OsString::from("notepad.exe"));
    let wide: Vec<_> = configuration.encode_wide().collect();
    // The executable token follows Windows argv[0] rules. Leave the remaining
    // command line untouched so the chosen editor interprets its own arguments.
    let mut program = Vec::new();
    let mut quoted = false;
    let mut end = 0;
    for (index, &unit) in wide.iter().enumerate() {
        if unit == u16::from(b'"') {
            quoted = !quoted;
        } else if !quoted && (unit == u16::from(b' ') || unit == u16::from(b'\t')) {
            break;
        } else {
            program.push(unit);
        }
        end = index + 1;
    }
    if program.is_empty() || quoted {
        return Err(Error::new(
            "invalid_argument",
            "Invalid editor command line",
            2,
        ));
    }
    let mut command = Command::new(OsString::from_wide(&program));
    command
        .raw_arg(OsString::from_wide(&wide[end..]))
        .arg(text)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = command
        .status()
        .map_err(|error| Error::new("io_error", format!("Could not run editor: {error}"), 1))?;
    if !status.success() {
        return Err(Error::new(
            "io_error",
            format!("Editor exited unsuccessfully: {status}"),
            1,
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn run(_text: &Path) -> Result<(), Error> {
    Err(Error::new(
        "io_error",
        "Editing comments requires Windows",
        1,
    ))
}
