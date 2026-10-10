use crate::{comment, comment::Extension, error::Error, name::Name, storage};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
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

pub fn columns(output: &mut impl Write, entries: &[Entry], color: bool) -> io::Result<()> {
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
                write_name(output, name, marker, color)?;
                writeln!(output, "{padding}{line}")?;
            } else {
                writeln!(output, "{}{line}", " ".repeat(width + 2))?;
            }
        }
    }
    Ok(())
}

pub fn long(output: &mut impl Write, entries: &[Entry], color: bool) -> io::Result<()> {
    for entry in entries {
        write_name(
            output,
            &entry.name,
            if entry.directory { "\\" } else { "" },
            color,
        )?;
        writeln!(output)?;
        for line in entry.comment.split('\n') {
            writeln!(output, "    {line}")?;
        }
        writeln!(output)?;
    }
    Ok(())
}

/// Collects valid records and diagnostics before rendering.
pub fn collect(directory: &Path, recursive: bool) -> Result<(Vec<Entry>, Vec<Error>), Error> {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    collect_directory(directory, recursive, &mut errors)?.flatten("", &mut entries);
    Ok((entries, errors))
}

struct Directory {
    entries: Vec<Entry>,
    children: Vec<(String, Directory)>,
}

impl Directory {
    fn flatten(self, prefix: &str, output: &mut Vec<Entry>) {
        for mut entry in self.entries {
            // Append the complete legacy name as text, never as path components.
            entry.name.insert_str(0, prefix);
            output.push(entry);
        }
        for (name, child) in self.children {
            child.flatten(&format!("{prefix}{name}\\"), output);
        }
    }

    fn into_tree(self) -> Vec<TreeNode> {
        let mut nodes: Vec<_> = self
            .entries
            .into_iter()
            .map(|entry| TreeNode {
                name: entry.name,
                name_utf16: entry.name_utf16,
                directory: entry.directory,
                comment: Some(entry.comment),
                children: Vec::new(),
            })
            .collect();
        for (name, child) in self.children {
            let children = child.into_tree();
            if children.is_empty() {
                continue;
            }
            let key = Name::new(&name);
            if let Some(node) = nodes
                .iter_mut()
                .find(|node| node.directory && key.matches(&node.name))
            {
                node.children = children;
            } else {
                nodes.push(TreeNode {
                    name_utf16: name.encode_utf16().chain(Some(0)).collect(),
                    name,
                    directory: true,
                    comment: None,
                    children,
                });
            }
        }
        nodes.sort_by(|a, b| {
            b.directory
                .cmp(&a.directory)
                .then_with(|| compare_names(&a.name_utf16, &b.name_utf16))
        });
        nodes
    }
}

pub struct TreeNode {
    name: String,
    name_utf16: Vec<u16>,
    directory: bool,
    comment: Option<String>,
    children: Vec<TreeNode>,
}

/// Uses the same traversal and diagnostics as the ordinary recursive list.
pub fn collect_tree(directory: &Path) -> Result<(Vec<TreeNode>, Vec<Error>), Error> {
    let mut errors = Vec::new();
    let nodes = collect_directory(directory, true, &mut errors)?.into_tree();
    Ok((nodes, errors))
}

pub fn tree(output: &mut impl Write, nodes: &[TreeNode], color: bool) -> io::Result<()> {
    render_tree(output, nodes, "", color)
}

fn render_tree(
    output: &mut impl Write,
    nodes: &[TreeNode],
    prefix: &str,
    color: bool,
) -> io::Result<()> {
    for (index, node) in nodes.iter().enumerate() {
        let last = index + 1 == nodes.len();
        let branch = if last { "└── " } else { "├── " };
        let continuation = if last { "    " } else { "│   " };
        let marker = if node.directory { "\\" } else { "" };
        write_tree_structure(output, prefix, color)?;
        write_tree_structure(output, branch, color)?;
        write_name(output, &node.name, marker, color)?;
        if let Some(comment) = &node.comment {
            let has_children = !node.children.is_empty();
            let child_guide = "│   ";
            let raw_width = node.name.width() + marker.len() + 2;
            let padding_len = if has_children {
                raw_width.saturating_sub(child_guide.width())
            } else {
                raw_width
            };
            let padding = " ".repeat(padding_len);
            for (index, line) in comment.split('\n').enumerate() {
                if index == 0 {
                    writeln!(output, "  {line}")?;
                } else {
                    write_tree_structure(output, prefix, color)?;
                    write_tree_structure(output, continuation, color)?;
                    if has_children {
                        write_tree_structure(output, child_guide, color)?;
                    }
                    writeln!(output, "{padding}{line}")?;
                }
            }
        } else {
            writeln!(output)?;
        }
        render_tree(
            output,
            &node.children,
            &format!("{prefix}{continuation}"),
            color,
        )?;
    }
    Ok(())
}

fn write_tree_structure(output: &mut impl Write, structure: &str, color: bool) -> io::Result<()> {
    for (index, segment) in structure.split(' ').enumerate() {
        if index > 0 {
            write!(output, " ")?;
        }
        if color && !segment.is_empty() {
            write!(output, "\x1b[36m{segment}\x1b[39m")?;
        } else {
            write!(output, "{segment}")?;
        }
    }
    Ok(())
}

fn write_name(output: &mut impl Write, name: &str, marker: &str, color: bool) -> io::Result<()> {
    if color {
        write!(output, "\x1b[36m{name}{marker}\x1b[39m")
    } else {
        write!(output, "{name}{marker}")
    }
}

fn collect_directory(
    directory: &Path,
    recursive: bool,
    errors: &mut Vec<Error>,
) -> Result<Directory, Error> {
    // Finish enumeration first: a failed directory contributes no partial contents.
    let mut directories = BTreeSet::new();
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
                directories.insert(Name::new(name));
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
    let file = directory.join("descript.ion");
    let bytes = match storage::read(&file) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(error);
            None
        }
    };
    let records = match &bytes {
        Some(bytes) => match comment::parse_for_list(bytes) {
            Ok((records, diagnostics)) => {
                errors.extend(diagnostics.into_iter().map(|error| error.at_file(&file)));
                records
            }
            Err(error) => {
                errors.push(error.at_file(&file));
                Vec::new()
            }
        },
        None => Vec::new(),
    };
    // Lookup uses the complete record name, never a path derived from that name.
    let mut entries: Vec<_> = records
        .into_iter()
        .map(|record| Entry {
            directory: directories.contains(&Name::new(record.name)),
            name_utf16: record.name.encode_utf16().chain(Some(0)).collect(),
            name: record.name.to_owned(),
            comment: record.comment.into_owned(),
            extension: record.extension,
        })
        .collect();
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| compare_names(&a.name_utf16, &b.name_utf16))
    });
    children.sort_by(|a, b| compare_names(&a.1, &b.1));
    let mut subdirectories = Vec::new();
    for (name, _, path) in children {
        match collect_directory(&path, true, errors) {
            Ok(child) => subdirectories.push((name, child)),
            Err(error) => errors.push(error),
        }
    }
    Ok(Directory {
        entries,
        children: subdirectories,
    })
}

fn compare_names(a: &[u16], b: &[u16]) -> Ordering {
    natural_cmp(a, b).then_with(|| a.cmp(b))
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
