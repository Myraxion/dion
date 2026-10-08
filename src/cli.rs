use crate::{comment, error::Error, listing, storage};
use std::{
    env,
    ffi::OsString,
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

pub fn run() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let json = args
        .iter()
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--json");
    match parse_args(&args).and_then(|args| execute(&args, json)) {
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

enum CommentSource {
    Stdin,
    File(PathBuf),
}

struct Arguments {
    positional: Vec<OsString>,
    source: Option<CommentSource>,
    long: bool,
    recursive: bool,
}

fn parse_args(args: &[OsString]) -> Result<Arguments, Error> {
    let mut positional = Vec::new();
    let mut after_separator = false;
    let mut json_seen = false;
    let mut source = None;
    let mut long = false;
    let mut recursive = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if after_separator {
            positional.push(arg.clone());
        } else if arg == "--" {
            after_separator = true;
        } else if arg == "--json" && !json_seen {
            json_seen = true;
        } else if (arg == "--long" || arg == "-l") && !long {
            long = true;
        } else if (arg == "--recursive" || arg == "-r") && !recursive {
            recursive = true;
        } else if (arg == "--stdin" || arg == "--comment-file") && source.is_none() {
            source = Some(if arg == "--stdin" {
                CommentSource::Stdin
            } else {
                let file = args.next().filter(|arg| {
                    !arg.to_str().is_some_and(|arg| arg.starts_with('-'))
                }).ok_or_else(|| {
                    Error::new("invalid_argument", "--comment-file requires a file; prefix a filename starting with - with ./", 2)
                })?;
                CommentSource::File(PathBuf::from(file))
            });
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
    Ok(Arguments {
        positional,
        source,
        long,
        recursive,
    })
}

fn execute(arguments: &Arguments, json: bool) -> Result<(), Error> {
    let args = &arguments.positional;
    if (arguments.long || arguments.recursive)
        && args.first().is_none_or(|command| command != "list")
    {
        return Err(Error::new(
            "invalid_argument",
            "--long and --recursive are only supported by list",
            2,
        ));
    }
    match args.first().and_then(|arg| arg.to_str()) {
        Some("get") if args.len() == 2 && arguments.source.is_none() => get(args, json),
        Some("remove") if args.len() == 2 && arguments.source.is_none() => remove(args, json),
        Some("list") if args.len() <= 2 && arguments.source.is_none() => {
            list(args, json, arguments.long, arguments.recursive)
        }
        Some("set") if args.len() == if arguments.source.is_some() { 2 } else { 3 } => {
            set(args, arguments.source.as_ref(), json)
        }
        _ => Err(Error::new(
            "invalid_argument",
            "Usage: dion [--json] get <path> | remove <path> | list [directory] [--long|-l] [--recursive|-r] | set <path> (<comment> | --stdin | --comment-file <file>)",
            2,
        )),
    }
}

fn remove(args: &[OsString], json: bool) -> Result<(), Error> {
    let path = absolute_path(&args[1])?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::new("invalid_argument", "Path must have a UTF-8 entry name", 2))?;
    let file = path
        .parent()
        .ok_or_else(|| Error::new("invalid_argument", "Path has no parent", 2))?
        .join("descript.ion");
    let original = storage::read(&file)?;
    let changed = match &original {
        Some(original) => {
            match comment::remove(original, name).map_err(|error| error.at_file(&file))? {
                comment::Removal::Unchanged => false,
                comment::Removal::Update(bytes) => {
                    storage::commit(&file, Some(original), &bytes)?;
                    true
                }
                comment::Removal::DeleteFile => {
                    storage::remove(&file, original)?;
                    true
                }
            }
        }
        None => false,
    };
    if json {
        write_json(
            &mut io::stdout().lock(),
            &serde_json::json!({"changed": changed}),
        )
        .map_err(|error| Error::new("io_error", error.to_string(), 1))?;
    }
    Ok(())
}

fn set(args: &[OsString], source: Option<&CommentSource>, json: bool) -> Result<(), Error> {
    let path = absolute_path(&args[1])?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::new("invalid_argument", "Path must have a UTF-8 entry name", 2))?;
    let body = match source {
        None => args[2]
            .to_str()
            .ok_or_else(|| Error::new("invalid_argument", "Comment must be UTF-8", 2))?
            .to_owned(),
        Some(source) => read_comment(source)?,
    };
    if body.trim().is_empty() || body.contains(['\0', '\u{4}']) {
        return Err(Error::new(
            "invalid_argument",
            "A nonblank comment without NUL or control character 04 is required",
            2,
        ));
    }
    let body = body.replace("\r\n", "\n").replace('\r', "\n");
    std::fs::symlink_metadata(&path).map_err(|error| Error::io(error, &path))?;
    let file = path
        .parent()
        .ok_or_else(|| Error::new("invalid_argument", "Path has no parent", 2))?
        .join("descript.ion");
    let original = storage::read(&file)?;
    let bytes =
        comment::set(original.as_deref(), name, &body).map_err(|error| error.at_file(&file))?;
    if let Some(bytes) = &bytes {
        storage::commit(&file, original.as_deref(), bytes)?;
    }
    if json {
        write_json(
            &mut io::stdout().lock(),
            &serde_json::json!({"changed": bytes.is_some()}),
        )
        .map_err(|error| Error::new("io_error", error.to_string(), 1))?;
    }
    Ok(())
}

fn read_comment(source: &CommentSource) -> Result<String, Error> {
    let bytes = match source {
        CommentSource::Stdin => {
            let mut bytes = Vec::new();
            io::stdin()
                .lock()
                .read_to_end(&mut bytes)
                .map_err(|error| Error::new("io_error", error.to_string(), 1))?;
            bytes
        }
        CommentSource::File(file) => std::fs::read(file).map_err(|error| Error::io(error, file))?,
    };
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    std::str::from_utf8(bytes).map(str::to_owned).map_err(|_| {
        let error = Error::new("invalid_encoding", "Comment input is not valid UTF-8", 1);
        match source {
            CommentSource::File(file) => error.at_file(file),
            CommentSource::Stdin => error,
        }
    })
}

fn list(args: &[OsString], json: bool, long: bool, recursive: bool) -> Result<(), Error> {
    let directory = normalize_verbatim_path(
        args.get(1)
            .map_or_else(|| PathBuf::from("."), PathBuf::from),
    );
    let entries = listing::collect(&directory, recursive)?;
    let mut output = io::stdout().lock();
    let result = if json {
        write_json(&mut output, &serde_json::json!({ "entries": entries }))
    } else if long {
        listing::long(&mut output, &entries)
    } else {
        listing::columns(&mut output, &entries)
    };
    result.map_err(|error| Error::new("io_error", error.to_string(), 1))
}

fn get(args: &[OsString], json: bool) -> Result<(), Error> {
    let path = absolute_path(&args[1])?;
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
                "name": record.name, "comment": record.comment, "extension": record.extension
            }),
        )
    } else {
        output.write_all(record.comment.as_bytes())
    };
    result.map_err(|error| Error::new("io_error", error.to_string(), 1))
}

fn absolute_path(argument: &OsString) -> Result<PathBuf, Error> {
    let path = PathBuf::from(argument);
    if path.as_os_str().is_empty() {
        return Err(Error::new("invalid_argument", "Path must not be empty", 2));
    }
    // Resolve dot components without following the target entry's symbolic link.
    std::path::absolute(&path)
        .map(normalize_verbatim_path)
        .map_err(|error| Error::io(error, &path))
}

fn normalize_verbatim_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::Component;
        if !matches!(path.components().next(), Some(Component::Prefix(prefix)) if prefix.kind().is_verbatim())
        {
            return path;
        }
        // absolute leaves verbatim paths unchanged. Normalize their dot components
        // lexically too, preserving the prefix and never resolving the final link.
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                _ => normalized.push(component.as_os_str()),
            }
        }
        normalized
    }
    #[cfg(not(windows))]
    path
}

fn write_json(output: &mut impl Write, value: &impl serde::Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")
}
