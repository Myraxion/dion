# Dion (descript.ion comment CLI)

Dion is a native Windows command-line tool for viewing, listing, setting, and removing file and folder comments stored in Total Commander's UTF-8 `descript.ion` format. Read the [Chinese README](README.md).

## Features

- One portable executable for Windows 10/11 x64; no Rust installation or additional runtime is required.
- View one comment, list comments in a directory, set or replace a comment, and explicitly remove a comment.
- List entries with directory-first natural sorting, aligned columns, long format, recursion, tree output, and optional name coloring.
- Handle Unicode names, emoji, spaces, and multiline comments while preserving indentation, leading and trailing spaces, and blank lines.
- Preserve unchanged record bytes and order. Setting the existing comment does not rewrite the file.
- Read comment text from arguments, stdin, UTF-8 files, or an external editor. JSON output and stable exit codes are available.
- Show help and program-generated diagnostics in Simplified Chinese or English.

Dion supports local Windows filesystems and single-writer use. Network shares are not covered by its compatibility guarantees. `descript.ion` files must have a UTF-8 BOM.

Record names are matched case-insensitively using the Windows ordinal comparison rules. This does not follow per-directory case-sensitive settings. See the [behavior contract](docs/spec.md) for details.

## Quick start

```text
dion help [command]
dion [--lang <en|zh-CN|auto>] [--json|-j] get <path>
dion [--lang <en|zh-CN|auto>] [--json|-j] set <path> (<comment> | --comment-file|-f <file> | --stdin|-i | --edit|-e)
dion [--lang <en|zh-CN|auto>] [--json|-j] list [directory] [--long|-l | --tree|-t] [--recursive|-r] [--all|-a] [--color auto|always|never]
dion [--lang <en|zh-CN|auto>] [--json|-j] remove <path>
```

Commands and aliases:

| Command | Aliases |
| --- | --- |
| `get` | `view`, `cat` |
| `list` | `ls` |
| `set` | none |
| `remove` | `rm`, `unset`, `del` |

All commands and aliases support `help <command>` and `<command> -h`. Options take separate arguments. Combined short options, attached values, and `=` values are not supported.

### Language

Use `--lang en`, `--lang zh-CN`, or `--lang auto`. The same values are accepted by `DION_LANG`. The command-line option takes precedence, followed by the environment variable, then the current Windows user interface language. `auto` uses the Windows language directly. Simplified Chinese is supported; other Windows languages default to English. Invalid or empty `DION_LANG` values are ignored. An invalid `--lang` value is an argument error.

Language affects program-generated help and diagnostics. It does not change comments, file names, JSON field names, error codes, or exit codes. In JSON errors, the human-readable `message` follows the selected language; scripts should branch on `code`.

### Commands

- **`get`** reads the comment for the specified file or folder from its parent directory's `descript.ion`.
- **`set`** sets or replaces an entry's comment. A nonempty comment requires the target to exist. Choose exactly one source: an argument, `--comment-file <file>`, `--stdin`, or `--edit`. Files and stdin use UTF-8; files may have an optional BOM. The editor is selected from `VISUAL`, `EDITOR`, or Windows Notepad. Clearing the editor text removes the comment.
- **`list`** lists comments inside a directory (the current directory by default). Directories appear first. `--long` uses one-column output; `--recursive` descends into subdirectories; `--tree` displays a hierarchy and enables recursion. Recursive listing skips directories named `$RECYCLE.BIN`, `System Volume Information`, `.git`, `node_modules`, `.venv`, `__pycache__`, `.pytest_cache`, `.next`, `.svn`, `.mypy_cache`, `.ruff_cache`, `.tox`, `.nox`, and `.parcel-cache`, matching complete names case-insensitively at every depth. `--all` (`-a`) includes them; by itself it does not enable recursion. Skipping these directories does not hide their own comments or produce errors. Recursive traversal also does not follow directory symlinks or junctions encountered below the starting directory. `--color auto|always|never` controls name coloring; JSON never contains color codes. Bad records and inaccessible description files are skipped and summarized; the starting directory must be accessible.
- **`remove`** explicitly removes the comment for an entry. The target itself is unchanged. Removing the last record deletes `descript.ion`.

### General options

- `--lang <en|zh-CN|auto>` selects the program language; `DION_LANG` sets a default.
- `--json` (`-j`) writes machine-readable JSON and stable exit codes. It does not change the output mode for help.
- `--help` (`-h`) shows help. Use `dion help <command>` for a command or alias.
- Place `--` before a path or comment that starts with `-`.

## Build and release

Install stable Rust and the MSVC build tools, then run the unified checks in PowerShell:

```powershell
.\scripts\check.ps1
```

This runs formatting, Clippy, the full test suite, and a release build. The executable is `target/release/dion.exe`; the MSVC runtime is statically linked. GitHub Actions runs the same checks on pushes and pull requests. Before releasing, update the version in `Cargo.toml` and `Cargo.lock`, prepare and confirm `docs/releases/<tag>.md`, then commit and push the changes. Pushing the corresponding `v*` tag creates a GitHub Release with those notes and the executable. Download releases from [GitHub Releases](https://github.com/Myraxion/dion/releases).

See the [behavior contract](docs/spec.md), [documentation index](docs/README.md), and [performance report](docs/benchmarks.md).

## License

Dion is available under either the [MIT License](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).
