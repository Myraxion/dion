use crate::{comment, comment::Extension, error::Error, name::Name, storage};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::Path,
    sync::OnceLock,
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
pub fn collect(
    directory: &Path,
    recursive: bool,
    include_excluded: bool,
) -> Result<(Vec<Entry>, Vec<Error>), Error> {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    collect_directory(directory, recursive, include_excluded, &mut errors)?
        .flatten("", &mut entries);
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
pub fn collect_tree(
    directory: &Path,
    include_excluded: bool,
) -> Result<(Vec<TreeNode>, Vec<Error>), Error> {
    let mut errors = Vec::new();
    let nodes = collect_directory(directory, true, include_excluded, &mut errors)?.into_tree();
    Ok((nodes, errors))
}

/// Builds the same list tree from indexed description-file candidates.
pub fn collect_indexed(
    directory: &Path,
    descriptions: &[std::path::PathBuf],
    include_excluded: bool,
) -> Result<(Vec<Entry>, Vec<Error>), Error> {
    let (tree, errors) = collect_indexed_directory(directory, descriptions, include_excluded)?;
    let mut entries = Vec::new();
    tree.flatten("", &mut entries);
    Ok((entries, errors))
}

/// Builds tree output from indexed description-file candidates.
pub fn collect_indexed_tree(
    directory: &Path,
    descriptions: &[std::path::PathBuf],
    include_excluded: bool,
) -> Result<(Vec<TreeNode>, Vec<Error>), Error> {
    let (tree, errors) = collect_indexed_directory(directory, descriptions, include_excluded)?;
    Ok((tree.into_tree(), errors))
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
    include_excluded: bool,
    errors: &mut Vec<Error>,
) -> Result<Directory, Error> {
    // Finish enumeration first: a failed directory contributes no partial contents.
    let mut directories = BTreeSet::new();
    let mut children = Vec::new();
    static DESCRIPTION_NAME: OnceLock<Name> = OnceLock::new();
    let description_name = DESCRIPTION_NAME.get_or_init(|| Name::new("descript.ion"));
    let mut has_description = false;
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
        let name = entry.file_name();
        if !has_description
            && name
                .to_str()
                .is_some_and(|name| description_name.matches(name))
        {
            has_description = true;
        }
        if is_directory {
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
                if !include_excluded && is_default_excluded_directory(&name) {
                    continue;
                }
                let key: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
                children.push((name, key, entry.path()));
            }
        }
    }
    let file = directory.join("descript.ion");
    let entries = if has_description {
        read_description(&file, &directories, errors)
    } else {
        Vec::new()
    };
    children.sort_by(|a, b| compare_names(&a.1, &b.1));
    let mut subdirectories = Vec::new();
    for (name, _, path) in children {
        match collect_directory(&path, true, include_excluded, errors) {
            Ok(child) => subdirectories.push((name, child)),
            Err(error) => errors.push(error),
        }
    }
    Ok(Directory {
        entries,
        children: subdirectories,
    })
}

struct IndexedDirectory {
    name: String,
    path: std::path::PathBuf,
    has_description: bool,
    children: Vec<IndexedDirectory>,
}

fn collect_indexed_directory(
    directory: &Path,
    descriptions: &[std::path::PathBuf],
    include_excluded: bool,
) -> Result<(Directory, Vec<Error>), Error> {
    let root = indexed_root(directory)?;
    let mut indexed = IndexedDirectory {
        name: String::new(),
        path: root.clone(),
        has_description: false,
        children: Vec::new(),
    };
    for description in descriptions {
        let Some(parent) = description.parent() else {
            continue;
        };
        let Some(components) = relative_directory_names(&root, parent)? else {
            continue;
        };
        if !include_excluded && components.iter().any(|name| is_default_excluded_directory(name))
        {
            continue;
        }
        if has_linked_directory(&root, &components)? {
            continue;
        }
        let mut branch = &mut indexed;
        for name in components {
            let index = branch
                .children
                .iter()
                .position(|child| child.name == name)
                .unwrap_or_else(|| {
                    branch.children.push(IndexedDirectory {
                        name: name.clone(),
                        path: branch.path.join(&name),
                        has_description: false,
                        children: Vec::new(),
                    });
                    branch.children.len() - 1
                });
            branch = &mut branch.children[index];
        }
        branch.has_description = true;
    }
    let mut errors = Vec::new();
    let tree = collect_indexed_branch(&indexed, include_excluded, &mut errors)?;
    Ok((tree, errors))
}

fn indexed_root(directory: &Path) -> Result<std::path::PathBuf, Error> {
    let absolute = std::path::absolute(directory).map_err(|error| Error::io(error, directory))?;
    #[cfg(windows)]
    {
        let path = absolute.to_str().ok_or_else(|| {
            Error::new("invalid_encoding", "Directory name is not valid UTF-8", 1)
                .at_file(directory)
        })?;
        let path = if let Some(path) = path.strip_prefix("\\\\?\\UNC\\") {
            format!("\\\\{path}")
        } else if let Some(path) = path.strip_prefix("\\\\?\\") {
            path.to_owned()
        } else {
            path.to_owned()
        };
        Ok(std::path::PathBuf::from(path))
    }
    #[cfg(not(windows))]
    {
        Ok(absolute)
    }
}

fn relative_directory_names(root: &Path, directory: &Path) -> Result<Option<Vec<String>>, Error> {
    let root_components: Vec<_> = root.components().collect();
    let directory_components: Vec<_> = directory.components().collect();
    if directory_components.len() < root_components.len() {
        return Ok(None);
    }
    for (left, right) in root_components.iter().zip(&directory_components) {
        let left = left.as_os_str().to_str().ok_or_else(|| {
            Error::new("invalid_encoding", "Directory name is not valid UTF-8", 1).at_file(root)
        })?;
        let right = right.as_os_str().to_str().ok_or_else(|| {
            Error::new("invalid_encoding", "Directory name is not valid UTF-8", 1)
                .at_file(directory)
        })?;
        if !Name::new(left).matches(right) {
            return Ok(None);
        }
    }
    directory_components[root_components.len()..]
        .iter()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| {
                    Error::new("invalid_encoding", "Directory name is not valid UTF-8", 1)
                        .at_file(directory)
                })
        })
        .collect::<Result<_, _>>()
        .map(Some)
}

fn has_linked_directory(root: &Path, components: &[String]) -> Result<bool, Error> {
    let mut path = root.to_owned();
    for component in components {
        path.push(component);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(_) => return Ok(false),
        };
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            if metadata.file_type().is_symlink_dir() {
                return Ok(true);
            }
        }
        #[cfg(not(windows))]
        if metadata.file_type().is_symlink() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn collect_indexed_branch(
    branch: &IndexedDirectory,
    include_excluded: bool,
    errors: &mut Vec<Error>,
) -> Result<Directory, Error> {
    let mut directories = BTreeSet::new();
    for entry in fs::read_dir(&branch.path).map_err(|error| Error::io(error, &branch.path))? {
        let entry = entry.map_err(|error| Error::io(error, &branch.path))?;
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
        if is_directory
            && let Some(name) = entry.file_name().to_str()
        {
            directories.insert(Name::new(name));
        }
    }
    let file = branch.path.join("descript.ion");
    let entries = if branch.has_description {
        read_description(&file, &directories, errors)
    } else {
        Vec::new()
    };
    let mut children = branch.children.iter().collect::<Vec<_>>();
    children.sort_by(|left, right| compare_names(&left.name_utf16(), &right.name_utf16()));
    let mut subdirectories = Vec::new();
    for child in children {
        if !include_excluded && is_default_excluded_directory(&child.name) {
            continue;
        }
        match collect_indexed_branch(child, include_excluded, errors) {
            Ok(directory) => subdirectories.push((child.name.clone(), directory)),
            Err(error) => errors.push(error),
        }
    }
    Ok(Directory {
        entries,
        children: subdirectories,
    })
}

impl IndexedDirectory {
    fn name_utf16(&self) -> Vec<u16> {
        self.name.encode_utf16().chain(Some(0)).collect()
    }
}

fn read_description(
    file: &Path,
    directories: &BTreeSet<Name>,
    errors: &mut Vec<Error>,
) -> Vec<Entry> {
    let bytes = match storage::read(file) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(error);
            None
        }
    };
    let mut entries = match bytes {
        Some(bytes) => match comment::parse_for_list(&bytes) {
            Ok((records, diagnostics)) => {
                errors.extend(diagnostics.into_iter().map(|error| error.at_file(file)));
                records
                    .into_iter()
                    .map(|record| Entry {
                        directory: directories.contains(&Name::new(record.name)),
                        name_utf16: record.name.encode_utf16().chain(Some(0)).collect(),
                        name: record.name.to_owned(),
                        comment: record.comment.into_owned(),
                        extension: record.extension,
                    })
                    .collect()
            }
            Err(error) => {
                errors.push(error.at_file(file));
                Vec::new()
            }
        },
        None => Vec::new(),
    };
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| compare_names(&a.name_utf16, &b.name_utf16))
    });
    entries
}

fn is_default_excluded_directory(name: &str) -> bool {
    static EXCLUDED: OnceLock<[Name; 14]> = OnceLock::new();
    let name = Name::new(name);
    EXCLUDED
        .get_or_init(|| {
            [
                Name::new("$RECYCLE.BIN"),
                Name::new("System Volume Information"),
                Name::new(".git"),
                Name::new("node_modules"),
                Name::new(".venv"),
                Name::new("__pycache__"),
                Name::new(".pytest_cache"),
                Name::new(".next"),
                Name::new(".svn"),
                Name::new(".mypy_cache"),
                Name::new(".ruff_cache"),
                Name::new(".tox"),
                Name::new(".nox"),
                Name::new(".parcel-cache"),
            ]
        })
        .contains(&name)
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
