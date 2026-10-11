use crate::{
    comment,
    comment::Extension,
    error::{Diagnostic, Error, LocalizedError},
    everything,
    i18n::Language,
    name::Name,
    storage,
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Write},
    path::Path,
    sync::OnceLock,
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Columns,
    Long,
    Tree,
}

pub enum Source<'a> {
    Scan,
    Everything { instance: Option<&'a str> },
}

pub struct Options<'a> {
    pub source: Source<'a>,
    pub format: Format,
    pub json: bool,
    pub recursive: bool,
    pub include_excluded: bool,
}

/// Collects before writing; skipped errors accompany the list and return status 1.
/// Fatal query or output errors are returned to the caller for stderr reporting.
pub fn list(
    directory: &Path,
    options: &Options<'_>,
    output: &mut impl Write,
    color: bool,
    language: Language,
) -> Result<u8, Error> {
    let (collected, errors) = match &options.source {
        Source::Scan => {
            let mut errors = Vec::new();
            let collected = collect_directory(
                directory,
                options.recursive || options.format == Format::Tree,
                options.include_excluded,
                &mut errors,
            )?;
            (collected, errors)
        }
        Source::Everything { instance } => {
            let descriptions = everything::find_descriptions(directory, *instance)?;
            collect_indexed_directory(directory, &descriptions, options.include_excluded)?
        }
    };
    let result = if options.format == Format::Tree && !options.json {
        let nodes = collected.into_tree();
        tree(output, &nodes, color)
            .and_then(|()| write_errors(output, &errors, !nodes.is_empty(), language))
    } else {
        // JSON always uses flat records, including when tree traversal was requested.
        let mut entries = Vec::new();
        collected.flatten("", &mut entries);
        if options.json {
            serde_json::to_writer(
                &mut *output,
                &ListOutput {
                    entries: &entries,
                    errors: errors
                        .iter()
                        .map(|error| error.localized(language))
                        .collect(),
                },
            )
            .map_err(io::Error::from)
            .and_then(|()| output.write_all(b"\n"))
        } else {
            let result = if options.format == Format::Long {
                long(output, &entries, color)
            } else {
                columns(output, &entries, color)
            };
            result.and_then(|()| write_errors(output, &errors, !entries.is_empty(), language))
        }
    };
    result.map_err(|error| Error::new("io_error", error.to_string(), 1))?;
    Ok(u8::from(!errors.is_empty()))
}

#[derive(serde::Serialize)]
struct ListOutput<'a> {
    entries: &'a [Entry],
    errors: Vec<LocalizedError<'a>>,
}

fn write_errors(
    output: &mut impl Write,
    errors: &[Error],
    has_entries: bool,
    language: Language,
) -> io::Result<()> {
    if !errors.is_empty() {
        if has_entries {
            writeln!(output)?;
        }
        if language.is_chinese() {
            writeln!(output, "错误（{}）：", errors.len())?;
        } else {
            writeln!(output, "Errors ({}):", errors.len())?;
        }
        for error in errors {
            error.write_text(output, language)?;
        }
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct Entry {
    name: String,
    comment: String,
    extension: Extension,
    #[serde(skip)]
    directory: bool,
    #[serde(skip)]
    name_utf16: Vec<u16>,
}

fn columns(output: &mut impl Write, entries: &[Entry], color: bool) -> io::Result<()> {
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

fn long(output: &mut impl Write, entries: &[Entry], color: bool) -> io::Result<()> {
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
        let mut directory_nodes: BTreeMap<_, _> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.directory)
            .map(|(index, node)| (Name::new(&node.name), index))
            .collect();
        for (name, child) in self.children {
            let children = child.into_tree();
            if children.is_empty() {
                continue;
            }
            let key = Name::new(&name);
            if let Some(&index) = directory_nodes.get(&key) {
                nodes[index].children = children;
            } else {
                directory_nodes.insert(key, nodes.len());
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

struct TreeNode {
    name: String,
    name_utf16: Vec<u16>,
    directory: bool,
    comment: Option<String>,
    children: Vec<TreeNode>,
}

fn tree(output: &mut impl Write, nodes: &[TreeNode], color: bool) -> io::Result<()> {
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
                        Error::new("invalid_encoding", Diagnostic::InvalidDirectoryEncoding, 1)
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
    name_utf16: Vec<u16>,
    path: std::path::PathBuf,
    has_description: bool,
    access: DirectoryAccess,
    children: BTreeMap<String, IndexedDirectory>,
}

enum DirectoryAccess {
    Readable,
    Skip,
    Failed(Error),
}

fn collect_indexed_directory(
    directory: &Path,
    descriptions: &[std::path::PathBuf],
    include_excluded: bool,
) -> Result<(Directory, Vec<Error>), Error> {
    let root = indexed_root(directory)?;
    let mut indexed = IndexedDirectory {
        name: String::new(),
        name_utf16: Vec::new(),
        path: root.clone(),
        has_description: false,
        access: DirectoryAccess::Readable,
        children: BTreeMap::new(),
    };
    for description in descriptions {
        let Some(parent) = description.parent() else {
            continue;
        };
        let Some(components) = relative_directory_names(&root, parent)? else {
            continue;
        };
        if !include_excluded
            && components
                .iter()
                .any(|name| is_default_excluded_directory(name))
        {
            continue;
        }
        let mut branch = &mut indexed;
        for name in components {
            let path = branch.path.join(&name);
            branch = branch
                .children
                .entry(name.clone())
                .or_insert_with(|| IndexedDirectory::new(name, path));
            if !matches!(branch.access, DirectoryAccess::Readable) {
                break;
            }
        }
        if matches!(branch.access, DirectoryAccess::Readable) {
            branch.has_description = true;
        }
    }
    let mut errors = Vec::new();
    let tree = collect_indexed_branch(indexed, &mut errors)?;
    Ok((tree, errors))
}

fn indexed_root(directory: &Path) -> Result<std::path::PathBuf, Error> {
    let absolute = std::path::absolute(directory).map_err(|error| Error::io(error, directory))?;
    #[cfg(windows)]
    {
        let path = absolute.to_str().ok_or_else(|| {
            Error::new("invalid_encoding", Diagnostic::InvalidDirectoryEncoding, 1)
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
            Error::new("invalid_encoding", Diagnostic::InvalidDirectoryEncoding, 1).at_file(root)
        })?;
        let right = right.as_os_str().to_str().ok_or_else(|| {
            Error::new("invalid_encoding", Diagnostic::InvalidDirectoryEncoding, 1)
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
                    Error::new("invalid_encoding", Diagnostic::InvalidDirectoryEncoding, 1)
                        .at_file(directory)
                })
        })
        .collect::<Result<_, _>>()
        .map(Some)
}

impl IndexedDirectory {
    fn new(name: String, path: std::path::PathBuf) -> Self {
        let access = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::FileTypeExt;
                    metadata.file_type().is_symlink_dir()
                };
                #[cfg(not(windows))]
                let linked = metadata.file_type().is_symlink();
                if linked {
                    DirectoryAccess::Skip
                } else {
                    DirectoryAccess::Readable
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => DirectoryAccess::Skip,
            Err(error) => DirectoryAccess::Failed(Error::io(error, &path)),
        };
        Self {
            name_utf16: name.encode_utf16().chain(Some(0)).collect(),
            name,
            path,
            has_description: false,
            access,
            children: BTreeMap::new(),
        }
    }
}

fn collect_indexed_branch(
    branch: IndexedDirectory,
    errors: &mut Vec<Error>,
) -> Result<Directory, Error> {
    match branch.access {
        DirectoryAccess::Readable => {}
        DirectoryAccess::Skip => {
            return Ok(Directory {
                entries: Vec::new(),
                children: Vec::new(),
            });
        }
        DirectoryAccess::Failed(error) => return Err(error),
    }
    let mut directories = BTreeSet::new();
    if branch.has_description {
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
            if is_directory && let Some(name) = entry.file_name().to_str() {
                directories.insert(Name::new(name));
            }
        }
    }
    let file = branch.path.join("descript.ion");
    let entries = if branch.has_description {
        read_description(&file, &directories, errors)
    } else {
        Vec::new()
    };
    let mut children = branch.children.into_values().collect::<Vec<_>>();
    children.sort_by(|left, right| compare_names(&left.name_utf16, &right.name_utf16));
    let mut subdirectories = Vec::new();
    for child in children {
        let name = child.name.clone();
        match collect_indexed_branch(child, errors) {
            Ok(directory) => subdirectories.push((name, directory)),
            Err(error) => errors.push(error),
        }
    }
    Ok(Directory {
        entries,
        children: subdirectories,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_failures_are_fatal_for_records_and_error_summaries() {
        let directory = tempfile::tempdir().unwrap();
        for bytes in [b"\xef\xbb\xbfentry note\n".as_slice(), b"missing BOM"] {
            fs::write(directory.path().join("descript.ion"), bytes).unwrap();
            for format in [Format::Columns, Format::Long, Format::Tree] {
                for json in [false, true] {
                    let options = Options {
                        source: Source::Scan,
                        format,
                        json,
                        recursive: false,
                        include_excluded: false,
                    };
                    let mut buffer = [];
                    let mut output = io::Cursor::new(buffer.as_mut_slice());
                    let error = list(
                        directory.path(),
                        &options,
                        &mut output,
                        false,
                        Language::English,
                    )
                    .expect_err("a full output buffer must fail");
                    assert_eq!(error.code, "io_error");
                    assert_eq!(error.exit_code, 1);
                }
            }
        }
    }
}
