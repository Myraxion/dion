use crate::{comment, comment::Extension, error::Error, storage};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    io::{self, Write},
    path::Path,
};
use unicode_width::UnicodeWidthStr;

#[derive(serde::Serialize)]
pub struct Entry {
    pub name: String,
    pub comment: String,
    pub extension: Extension,
    #[serde(skip)]
    pub directory: bool,
    #[serde(skip)]
    name_utf16: Vec<u16>,
}

pub fn columns(output: &mut impl Write, entries: &[Entry]) -> io::Result<()> {
    let width = entries
        .iter()
        .map(|entry| entry.name.width() + usize::from(entry.directory))
        .max()
        .unwrap_or(0);
    for entry in entries {
        let name = &entry.name;
        let marker = if entry.directory { "\\" } else { "" };
        let padding = " ".repeat(width - name.width() - marker.len() + 2);
        for (index, line) in entry.comment.split('\n').enumerate() {
            if index == 0 {
                writeln!(output, "{name}{marker}{padding}{line}")?;
            } else {
                writeln!(output, "{}{line}", " ".repeat(width + 2))?;
            }
        }
    }
    Ok(())
}

pub fn long(output: &mut impl Write, entries: &[Entry]) -> io::Result<()> {
    for entry in entries {
        writeln!(
            output,
            "{}{}",
            entry.name,
            if entry.directory { "\\" } else { "" }
        )?;
        for line in entry.comment.split('\n') {
            writeln!(output, "    {line}")?;
        }
        writeln!(output)?;
    }
    Ok(())
}

/// Collects all records before rendering, so a later read failure emits no stdout.
pub fn collect(directory: &Path, recursive: bool) -> Result<Vec<Entry>, Error> {
    let mut entries = Vec::new();
    collect_directory(directory, "", recursive, &mut entries)?;
    Ok(entries)
}

fn collect_directory(
    directory: &Path,
    prefix: &str,
    recursive: bool,
    output: &mut Vec<Entry>,
) -> Result<(), Error> {
    let file = directory.join("descript.ion");
    let bytes = storage::read(&file)?;
    let records = match &bytes {
        Some(bytes) => comment::parse(bytes).map_err(|error| error.at_file(&file))?,
        None => Vec::new(),
    };
    if records.is_empty() && !recursive {
        return Ok(());
    }
    let mut directories = HashSet::new();
    let mut children = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| Error::io(error, directory))? {
        let entry = entry.map_err(|error| Error::io(error, directory))?;
        let kind = entry
            .file_type()
            .map_err(|error| Error::io(error, &entry.path()))?;
        #[cfg(windows)]
        let is_directory = {
            use std::os::windows::fs::FileTypeExt;
            kind.is_dir() || kind.is_symlink_dir()
        };
        #[cfg(not(windows))]
        let is_directory = kind.is_dir();
        if is_directory {
            let name = entry.file_name();
            if let Some(name) = name.to_str() {
                directories.insert(name.to_lowercase());
            }
            // Windows file types classify both directory symlinks and Junctions
            // as symlinks; other directory reparse points remain traversable.
            if recursive && kind.is_dir() {
                let name = name
                    .to_str()
                    .ok_or_else(|| {
                        Error::new("invalid_encoding", "Directory name is not valid UTF-8", 1)
                            .at_file(&entry.path())
                    })?
                    .to_owned();
                let key: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
                children.push((name, key, entry.path()));
            }
        }
    }
    // Lookup uses the complete record name, never a path derived from that name.
    let mut entries: Vec<_> = records
        .into_iter()
        .map(|record| Entry {
            directory: directories.contains(&record.name.to_lowercase()),
            name_utf16: record.name.encode_utf16().chain(Some(0)).collect(),
            name: record.name.to_owned(),
            comment: record.comment.into_owned(),
            extension: record.extension,
        })
        .collect();
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| natural_cmp(&a.name_utf16, &b.name_utf16))
            .then_with(|| a.name_utf16.cmp(&b.name_utf16))
    });
    for mut entry in entries {
        // Append the complete legacy name as text, never as path components.
        entry.name.insert_str(0, prefix);
        output.push(entry);
    }
    children.sort_by(|a, b| natural_cmp(&a.1, &b.1).then_with(|| a.1.cmp(&b.1)));
    for (name, _, path) in children {
        collect_directory(&path, &format!("{prefix}{name}\\"), true, output)?;
    }
    Ok(())
}

#[cfg(windows)]
fn natural_cmp(a: &[u16], b: &[u16]) -> Ordering {
    #[link(name = "shlwapi")]
    unsafe extern "system" {
        fn StrCmpLogicalW(a: *const u16, b: *const u16) -> i32;
    }
    // SAFETY: Both slices are live UTF-16 buffers terminated with NUL.
    unsafe { StrCmpLogicalW(a.as_ptr(), b.as_ptr()) }.cmp(&0)
}

#[cfg(not(windows))]
fn natural_cmp(a: &[u16], b: &[u16]) -> Ordering {
    a.cmp(b)
}
