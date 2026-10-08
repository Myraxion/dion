use crate::{comment::Record, error::Error};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    io::{self, Write},
    path::Path,
};
use unicode_width::UnicodeWidthStr;

pub struct Entry<'a> {
    pub record: Record<'a>,
    pub directory: bool,
    name_utf16: Vec<u16>,
}

pub fn columns(output: &mut impl Write, entries: &[Entry<'_>]) -> io::Result<()> {
    let width = entries
        .iter()
        .map(|entry| entry.record.name.width() + usize::from(entry.directory))
        .max()
        .unwrap_or(0);
    for entry in entries {
        let name = entry.record.name;
        let marker = if entry.directory { "\\" } else { "" };
        let padding = " ".repeat(width - name.width() - marker.len() + 2);
        for (index, line) in entry.record.comment.split('\n').enumerate() {
            if index == 0 {
                writeln!(output, "{name}{marker}{padding}{line}")?;
            } else {
                writeln!(output, "{}{line}", " ".repeat(width + 2))?;
            }
        }
    }
    Ok(())
}

pub fn long(output: &mut impl Write, entries: &[Entry<'_>]) -> io::Result<()> {
    for entry in entries {
        writeln!(
            output,
            "{}{}",
            entry.record.name,
            if entry.directory { "\\" } else { "" }
        )?;
        for line in entry.record.comment.split('\n') {
            writeln!(output, "    {line}")?;
        }
        writeln!(output)?;
    }
    Ok(())
}

/// Orders only the supplied records; directory entries provide type information.
pub fn ordered<'a>(directory: &Path, records: Vec<Record<'a>>) -> Result<Vec<Entry<'a>>, Error> {
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let mut directories = HashSet::new();
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
        if is_directory && let Some(name) = entry.file_name().to_str() {
            directories.insert(name.to_lowercase());
        }
    }
    // Lookup uses the complete record name, never a path derived from that name.
    let mut entries: Vec<_> = records
        .into_iter()
        .map(|record| Entry {
            directory: directories.contains(&record.name.to_lowercase()),
            name_utf16: record.name.encode_utf16().chain(Some(0)).collect(),
            record,
        })
        .collect();
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| natural_cmp(&a.name_utf16, &b.name_utf16))
            .then_with(|| a.name_utf16.cmp(&b.name_utf16))
    });
    Ok(entries)
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
