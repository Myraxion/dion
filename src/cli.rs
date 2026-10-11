use crate::{
    comment,
    error::{Diagnostic, Error},
    help,
    i18n::Language,
    listing, modification,
    name::Name,
    storage, terminal,
};
use std::{
    env,
    ffi::OsString,
    io::{self, IsTerminal, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

pub fn run() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let (json, language) = output_options(&args);
    match parse_args(&args).and_then(|args| execute(&args, args.json, language)) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let mut stderr = io::stderr().lock();
            if json {
                let _ = write_json(
                    &mut stderr,
                    &serde_json::json!({ "error": error.localized(language) }),
                );
            } else {
                let _ = error.write_text(&mut stderr, language);
            }
            ExitCode::from(error.exit_code)
        }
    }
}

enum CommentSource {
    Stdin,
    File(PathBuf),
    Edit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

struct Arguments {
    json: bool,
    positional: Vec<OsString>,
    source: Option<CommentSource>,
    long: bool,
    recursive: bool,
    tree: bool,
    include_excluded: bool,
    help: bool,
    color: Option<ColorMode>,
    everything: bool,
    everything_instance: Option<String>,
}

fn parse_args(args: &[OsString]) -> Result<Arguments, Error> {
    let mut positional = Vec::new();
    let mut json_seen = false;
    let mut source = None;
    let mut long = false;
    let mut recursive = false;
    let mut tree = false;
    let mut include_excluded = false;
    let mut help = false;
    let mut color = None;
    let mut use_everything = false;
    let mut everything_instance = None;
    let mut language = None;
    for token in argument_tokens(args) {
        let (arg, value) = match token {
            ArgumentToken::Positional(arg) => {
                positional.push(arg.clone());
                continue;
            }
            ArgumentToken::Option(arg, value) => (arg, value),
        };
        if (arg == "--json" || arg == "-j") && !json_seen {
            json_seen = true;
        } else if arg == "--lang" && language.is_none() {
            let value = value
                .and_then(|value| value.to_str())
                .ok_or_else(|| Error::new("invalid_argument", Diagnostic::InvalidLanguage, 2))?;
            language =
                Some(Language::resolve_option(value).ok_or_else(|| {
                    Error::new("invalid_argument", Diagnostic::InvalidLanguage, 2)
                })?);
        } else if (arg == "--help" || arg == "-h") && !help {
            help = true;
        } else if (arg == "--long" || arg == "-l") && !long {
            long = true;
        } else if (arg == "--recursive" || arg == "-r") && !recursive {
            recursive = true;
        } else if (arg == "--tree" || arg == "-t") && !tree {
            tree = true;
        } else if arg == "--everything" && !use_everything {
            use_everything = true;
        } else if arg == "--everything-instance" && everything_instance.is_none() {
            let value = value
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    Error::new(
                        "invalid_argument",
                        Diagnostic::EverythingInstanceRequiresAName,
                        2,
                    )
                })?;
            everything_instance = Some(value.to_owned());
        } else if (arg == "--all" || arg == "-a") && !include_excluded {
            include_excluded = true;
        } else if arg == "--color" && color.is_none() {
            let value = value
                .and_then(|value| value.to_str())
                .ok_or_else(|| Error::new("invalid_argument", Diagnostic::InvalidColor, 2))?;
            color = Some(match value {
                "auto" => ColorMode::Auto,
                "always" => ColorMode::Always,
                "never" => ColorMode::Never,
                _ => {
                    return Err(Error::new("invalid_argument", Diagnostic::InvalidColor, 2));
                }
            });
        } else if (arg == "--stdin"
            || arg == "-i"
            || arg == "--comment-file"
            || arg == "-f"
            || arg == "--edit"
            || arg == "-e")
            && source.is_none()
        {
            source = Some(if arg == "--stdin" || arg == "-i" {
                CommentSource::Stdin
            } else if arg == "--edit" || arg == "-e" {
                CommentSource::Edit
            } else {
                let file = value
                    .filter(|arg| !arg.to_str().is_some_and(|arg| arg.starts_with('-')))
                    .ok_or_else(|| {
                        Error::new("invalid_argument", Diagnostic::CommentFileRequired, 2)
                    })?;
                CommentSource::File(PathBuf::from(file))
            });
        } else if arg.to_str().is_some_and(|arg| arg.starts_with('-')) {
            return Err(Error::new(
                "invalid_argument",
                Diagnostic::UnknownOrRepeatedOption,
                2,
            ));
        } else {
            positional.push(arg.clone());
        }
    }
    if tree && long {
        return Err(Error::new(
            "invalid_argument",
            Diagnostic::TreeAndLongAreMutuallyExclusive,
            2,
        ));
    }
    Ok(Arguments {
        json: json_seen,
        positional,
        source,
        long,
        recursive,
        tree,
        include_excluded,
        help,
        color,
        everything: use_everything,
        everything_instance,
    })
}

fn execute(arguments: &Arguments, json: bool, language: Language) -> Result<u8, Error> {
    let args = &arguments.positional;
    let command = args.first().and_then(|arg| arg.to_str()).map(command_name);
    if arguments.help || args.first().is_some_and(|arg| arg == "help") {
        let topic = if args.first().is_some_and(|arg| arg == "help") {
            if args.len() > 2 {
                return Err(Error::new("invalid_argument", Diagnostic::HelpUsage, 2));
            }
            args.get(1)
        } else {
            args.first()
        };
        let text = match topic {
            None => Some(help::overview(language)),
            Some(topic) => topic
                .to_str()
                .map(command_name)
                .and_then(|command| help::command_for(command, language)),
        }
        .ok_or_else(|| Error::new("invalid_argument", Diagnostic::UnknownHelpCommand, 2))?;
        return io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .map(|()| 0)
            .map_err(|error| Error::new("io_error", error.to_string(), 1));
    }
    if (arguments.long
        || arguments.recursive
        || arguments.tree
        || arguments.include_excluded
        || arguments.color.is_some()
        || arguments.everything
        || arguments.everything_instance.is_some())
        && command != Some("list")
    {
        return Err(Error::new(
            "invalid_argument",
            Diagnostic::ListOptionsOnly,
            2,
        ));
    }
    if arguments.everything_instance.is_some() && !arguments.everything {
        return Err(Error::new(
            "invalid_argument",
            Diagnostic::EverythingInstanceRequiresEverything,
            2,
        ));
    }
    if arguments.everything && !arguments.recursive && !arguments.tree {
        return Err(Error::new(
            "invalid_argument",
            Diagnostic::EverythingRequiresRecursiveOrTree,
            2,
        ));
    }
    match command {
        Some("get") if args.len() == 2 && arguments.source.is_none() => get(args, json).map(|()| 0),
        Some("remove") if args.len() == 2 && arguments.source.is_none() => {
            remove(args, json).map(|()| 0)
        }
        Some("list") if args.len() <= 2 && arguments.source.is_none() => {
            list(arguments, json, language)
        }
        Some("set") if args.len() == if arguments.source.is_some() { 2 } else { 3 } => {
            set(args, arguments.source.as_ref(), json).map(|()| 0)
        }
        _ => Err(Error::new("invalid_argument", Diagnostic::CommandUsage, 2)),
    }
}

enum ArgumentToken<'a> {
    Positional(&'a OsString),
    Option(&'a OsString, Option<&'a OsString>),
}

/// Shares value consumption between parsing and diagnostics after an earlier error.
fn argument_tokens(args: &[OsString]) -> impl Iterator<Item = ArgumentToken<'_>> {
    let mut args = args.iter();
    let mut after_separator = false;
    let mut value_options = [false; 3];
    let mut source_seen = false;
    std::iter::from_fn(move || {
        loop {
            let arg = args.next()?;
            if after_separator {
                return Some(ArgumentToken::Positional(arg));
            }
            if arg == "--" {
                after_separator = true;
                continue;
            }
            let Some(name) = arg.to_str().filter(|name| name.starts_with('-')) else {
                return Some(ArgumentToken::Positional(arg));
            };
            let value_index = match name {
                "--lang" => Some(0),
                "--color" => Some(1),
                "--everything-instance" => Some(2),
                _ => None,
            };
            let takes_value = if let Some(index) = value_index {
                !std::mem::replace(&mut value_options[index], true)
            } else if matches!(
                name,
                "--stdin" | "-i" | "--edit" | "-e" | "--comment-file" | "-f"
            ) {
                !std::mem::replace(&mut source_seen, true)
                    && matches!(name, "--comment-file" | "-f")
            } else {
                false
            };
            let value = if takes_value { args.next() } else { None };
            return Some(ArgumentToken::Option(arg, value));
        }
    })
}

fn output_options(args: &[OsString]) -> (bool, Language) {
    let mut json = false;
    let mut language = None;
    let mut language_seen = false;
    for token in argument_tokens(args) {
        if let ArgumentToken::Option(arg, value) = token {
            if arg == "--json" || arg == "-j" {
                json = true;
            } else if arg == "--lang" && !language_seen {
                language_seen = true;
                language = value
                    .and_then(|value| value.to_str())
                    .and_then(Language::resolve_option);
            }
        }
    }
    (json, language.unwrap_or_else(Language::from_environment))
}

fn command_name(name: &str) -> &str {
    match name {
        "view" | "cat" => "get",
        "ls" => "list",
        "rm" | "unset" | "del" => "remove",
        _ => name,
    }
}

fn remove(args: &[OsString], json: bool) -> Result<(), Error> {
    let path = absolute_path(&args[1])?;
    let changed = modification::Target::new(&path)?.remove()?;
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
    let target = modification::Target::new(&path)?;
    let changed = match source {
        Some(CommentSource::Edit) => target.edit()?,
        None => {
            let body = args[2].to_str().ok_or_else(|| {
                Error::new("invalid_argument", Diagnostic::InvalidCommentEncoding, 2)
            })?;
            target.set(body)?
        }
        Some(source) => target.set(&read_comment(source)?)?,
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
        CommentSource::Edit => {
            return Err(Error::new(
                "invalid_argument",
                Diagnostic::EditingRequiresATargetPath,
                2,
            ));
        }
    };
    comment::decode_input(&bytes).map_err(|error| match source {
        CommentSource::File(file) => error.at_file(file),
        CommentSource::Stdin => error,
        CommentSource::Edit => error,
    })
}

fn list(arguments: &Arguments, json: bool, language: Language) -> Result<u8, Error> {
    let args = &arguments.positional;
    let color_mode = arguments.color.unwrap_or(ColorMode::Auto);
    let directory = normalize_verbatim_path(
        args.get(1)
            .map_or_else(|| PathBuf::from("."), PathBuf::from),
    );
    let source = if arguments.everything {
        listing::Source::Everything {
            instance: arguments.everything_instance.as_deref(),
        }
    } else {
        listing::Source::Scan
    };
    let format = if arguments.tree {
        listing::Format::Tree
    } else if arguments.long {
        listing::Format::Long
    } else {
        listing::Format::Columns
    };
    let options = listing::Options {
        source,
        format,
        json,
        recursive: arguments.recursive,
        include_excluded: arguments.include_excluded,
    };
    let (use_color, _console_mode) = prepare_color(color_mode, json);
    listing::list(
        &directory,
        &options,
        &mut io::stdout().lock(),
        use_color,
        language,
    )
}

fn prepare_color(color_mode: ColorMode, json: bool) -> (bool, Option<terminal::ConsoleModeGuard>) {
    if json || color_mode == ColorMode::Never {
        return (false, None);
    }
    if color_mode == ColorMode::Auto
        && (env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
            || !io::stdout().is_terminal())
    {
        return (false, None);
    }
    let console_mode = terminal::enable_virtual_terminal_processing();
    (
        color_mode == ColorMode::Always || console_mode.is_some(),
        console_mode,
    )
}

fn get(args: &[OsString], json: bool) -> Result<(), Error> {
    let path = absolute_path(&args[1])?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::new("invalid_argument", Diagnostic::InvalidEntryName, 2))?;
    let file = path
        .parent()
        .ok_or_else(|| Error::new("invalid_argument", Diagnostic::PathHasNoParent, 2))?
        .join("descript.ion");
    let not_found = || Error::new("not_found", Diagnostic::CommentNotFound, 3).at_file(&file);
    let bytes = storage::read(&file)?.ok_or_else(not_found)?;
    let records = comment::parse(&bytes).map_err(|error| error.at_file(&file))?;
    let key = Name::new(name);
    let record = records
        .iter()
        .find(|record| key.matches(record.name))
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
    result
        .and_then(|()| output.flush())
        .map_err(|error| Error::new("io_error", error.to_string(), 1))
}

fn absolute_path(argument: &OsString) -> Result<PathBuf, Error> {
    let path = PathBuf::from(argument);
    if path.as_os_str().is_empty() {
        return Err(Error::new(
            "invalid_argument",
            Diagnostic::PathMustNotBeEmpty,
            2,
        ));
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
