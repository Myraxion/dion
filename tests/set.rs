use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;

fn set(directory: &Path, name: &str, body: &str, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
    command.current_dir(directory).env("DION_LANG", "en");
    if json {
        command.arg("--json");
    }
    command.args(["set", "--", name, body]).output().unwrap()
}

fn fixture(bytes: Option<&[u8]>) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("照片 😀.txt"), b"entry").unwrap();
    if let Some(bytes) = bytes {
        fs::write(directory.path().join("descript.ion"), bytes).unwrap();
    }
    directory
}

fn source_set(directory: &Path, source: &str, bytes: &[u8]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
    command.current_dir(directory).env("DION_LANG", "en").args([
        "--json",
        "set",
        "照片 😀.txt",
        source,
    ]);
    if source == "--comment-file" || source == "-f" {
        fs::write(directory.join("body.txt"), bytes).unwrap();
        command.arg("body.txt");
        command.output().unwrap()
    } else {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        child.wait_with_output().unwrap()
    }
}

#[test]
fn stdin_and_file_accept_optional_bom_preserve_unicode_and_tail_breaks() {
    for source in ["--stdin", "-i", "--comment-file", "-f"] {
        for prefix in ["", "\u{feff}"] {
            let directory = fixture(None);
            let input = format!("{prefix}  中文\\n\\folder\r\n\tline\r\n\r\n");
            changed(
                &source_set(directory.path(), source, input.as_bytes()),
                true,
            );
            let result = Command::new(env!("CARGO_BIN_EXE_dion"))
                .current_dir(directory.path())
                .args(["get", "照片 😀.txt"])
                .output()
                .unwrap();
            assert_eq!(result.status.code(), Some(0));
            assert!(result.stderr.is_empty());
            assert_eq!(result.stdout, "  中文\\n\\folder\n\tline\n\n".as_bytes());
        }
    }
}

#[test]
fn normalized_source_body_keeps_original_bytes_and_timestamp_when_equal() {
    for source in ["--stdin", "-i", "--comment-file", "-f"] {
        for prefix in ["", "\u{feff}"] {
            let original =
                "\u{feff}\"照片 😀.txt\"   中文\\\\n\\\\folder\\n\tline\\n\\n\u{4}\u{c2}\r";
            let directory = fixture(Some(original.as_bytes()));
            let file = directory.path().join("descript.ion");
            let before = fs::read(&file).unwrap();
            let timestamp = fs::metadata(&file).unwrap().modified().unwrap();
            changed(
                &source_set(
                    directory.path(),
                    source,
                    format!("{prefix}  中文\\n\\folder\r\n\tline\r\r").as_bytes(),
                ),
                false,
            );
            assert_eq!(fs::read(&file).unwrap(), before);
            assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), timestamp);
        }
    }
}

#[test]
fn input_sources_are_required_and_mutually_exclusive() {
    let directory = fixture(None);
    for args in [
        vec!["set", "照片 😀.txt"],
        vec!["set", "照片 😀.txt", "body", "--stdin"],
        vec!["set", "照片 😀.txt", "body", "--comment-file", "body.txt"],
        vec![
            "set",
            "照片 😀.txt",
            "--stdin",
            "--comment-file",
            "body.txt",
        ],
        vec!["set", "照片 😀.txt", "--stdin", "--stdin"],
        vec!["set", "照片 😀.txt", "--comment-file"],
        vec!["set", "照片 😀.txt", "--comment-file", "--stdin"],
        vec!["set", "照片 😀.txt", "--comment-file", "--json"],
        vec!["set", "照片 😀.txt", "--comment-file", "--comment-file"],
        vec![
            "set",
            "照片 😀.txt",
            "--comment-file",
            "body.txt",
            "--comment-file",
            "body.txt",
        ],
        vec!["get", "照片 😀.txt", "--stdin"],
        vec!["list", "--comment-file", "body.txt"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_dion"))
            .current_dir(directory.path())
            .arg("--json")
            .args(args)
            .output()
            .unwrap();
        rejected(&result, 2, "invalid_argument");
        assert!(!directory.path().join("descript.ion").exists());
    }
}

#[test]
fn invalid_stream_and_file_input_preserves_existing_bytes() {
    let original = "\u{feff}\r\n\"照片 😀.txt\" old\r\n".as_bytes();
    for source in ["--stdin", "--comment-file"] {
        for bytes in [None, Some(original)] {
            let directory = fixture(bytes);
            for (input, exit, code) in [
                (b"\xff".as_slice(), 1, "invalid_encoding"),
                (b"\xef\xbb\xbf\xff", 1, "invalid_encoding"),
                (b"", 2, "invalid_argument"),
                (b"\xef\xbb\xbf \t\r\n", 2, "invalid_argument"),
                (b"bad\0body", 2, "invalid_argument"),
                (b"bad\x04body", 2, "invalid_argument"),
            ] {
                let error = rejected(&source_set(directory.path(), source, input), exit, code);
                if source == "--comment-file" && code == "invalid_encoding" {
                    assert_eq!(error["error"]["file"], "body.txt");
                }
                let file = directory.path().join("descript.ion");
                if let Some(bytes) = bytes {
                    assert_eq!(fs::read(file).unwrap(), bytes);
                } else {
                    assert!(!file.exists());
                }
            }
        }
    }
}

#[test]
fn missing_comment_input_file_preserves_existing_description() {
    let original = "\u{feff}\r\n\"照片 😀.txt\" old\r\n".as_bytes();
    let directory = fixture(Some(original));
    let result = Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory.path())
        .args([
            "--json",
            "set",
            "照片 😀.txt",
            "--comment-file",
            "missing.txt",
        ])
        .output()
        .unwrap();
    rejected(&result, 1, "io_error");
    assert_eq!(
        fs::read(directory.path().join("descript.ion")).unwrap(),
        original
    );
}

#[test]
fn multiline_encoded_length_includes_escapes_extension_and_crlf() {
    // Quoted Unicode name + separator + CRLF = 20, TC marker = 3,
    // escaped backslash and LF = 4: leaves 4069 ASCII bytes.
    for source in ["argv", "--stdin", "--comment-file"] {
        for bytes in [
            None,
            Some(b"\xef\xbb\xbf\r\nother raw\x04unknown\n".as_slice()),
        ] {
            let directory = fixture(bytes);
            let body = format!("{}\\\n", "x".repeat(4069));
            let run = |body: &str| {
                if source == "argv" {
                    set(directory.path(), "照片 😀.txt", body, true)
                } else {
                    source_set(directory.path(), source, body.as_bytes())
                }
            };
            changed(&run(&body), true);
            let file = directory.path().join("descript.ion");
            let before = fs::read(&file).unwrap();
            assert_eq!(before.len(), bytes.map_or(5, |bytes| bytes.len()) + 4096);
            rejected(&run(&(body + "x")), 1, "invalid_format");
            assert_eq!(fs::read(&file).unwrap(), before);
        }
    }
}

#[test]
fn source_updates_preserve_other_records() {
    for source in ["--stdin", "--comment-file"] {
        let original = "\u{feff}\r\nother raw\\n\u{4}unknown\r\"照片 😀.txt\" old\n\r\nlast tail";
        let directory = fixture(Some(original.as_bytes()));
        changed(
            &source_set(directory.path(), source, "新\r\n正文\\n\n".as_bytes()),
            true,
        );
        assert_eq!(fs::read(directory.path().join("descript.ion")).unwrap(),
            "\u{feff}\r\nother raw\\n\u{4}unknown\r\"照片 😀.txt\" 新\\n正文\\\\n\\n\u{4}\u{c2}\r\n\r\nlast tail".as_bytes());
    }
}

#[test]
fn sources_reject_unknown_target_extensions_even_when_body_is_equal() {
    for source in ["--stdin", "--comment-file"] {
        let original = "\u{feff}\"照片 😀.txt\" old\u{4}unknown\n";
        let directory = fixture(Some(original.as_bytes()));
        rejected(
            &source_set(directory.path(), source, b"old"),
            1,
            "unknown_extension",
        );
        assert_eq!(
            fs::read(directory.path().join("descript.ion")).unwrap(),
            original.as_bytes()
        );
    }
}

fn changed(result: &Output, expected: bool) {
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    assert!(result.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        serde_json::json!({"changed": expected})
    );
}

#[test]
fn creates_hidden_utf8_comment_preserving_whitespace_and_literal_backslashes() {
    let directory = fixture(None);
    let result = set(directory.path(), "照片 😀.txt", r"  中文\n\path  ", true);
    changed(&result, true);
    let file = directory.path().join("descript.ion");
    assert_eq!(
        fs::read(&file).unwrap(),
        "\u{feff}\r\n\"照片 😀.txt\"   中文\\n\\path  \r\n".as_bytes()
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_ne!(fs::metadata(&file).unwrap().file_attributes() & 2, 0);
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn updates_in_place_and_appends_without_rewriting_other_bytes() {
    let original = "\u{feff}\r\nforeign raw\\n\u{4}other\n\"照片 😀.txt\" old\r\n\r\nlast   tail";
    let directory = fixture(Some(original.as_bytes()));
    changed(&set(directory.path(), "照片 😀.txt", "new", true), true);
    let file = directory.path().join("descript.ion");
    assert_eq!(
        fs::read(&file).unwrap(),
        "\u{feff}\r\nforeign raw\\n\u{4}other\n\"照片 😀.txt\" new\r\n\r\nlast   tail".as_bytes()
    );
    fs::create_dir(directory.path().join("folder")).unwrap();
    let result = set(directory.path(), "folder", "directory comment", false);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    assert_eq!(fs::read(&file).unwrap(), "\u{feff}\r\nforeign raw\\n\u{4}other\n\"照片 😀.txt\" new\r\n\r\nlast   tail\r\nfolder directory comment\r\n".as_bytes());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
}

#[test]
fn dot_directory_paths_set_the_directory_entry_in_its_parent() {
    let directory = fixture(None);
    let folder = directory.path().join("folder");
    fs::create_dir(&folder).unwrap();
    changed(&set(directory.path(), "folder/.", "first", true), true);
    assert!(!folder.join("descript.ion").exists());
    assert_eq!(
        fs::read(directory.path().join("descript.ion")).unwrap(),
        b"\xef\xbb\xbf\r\nfolder first\r\n"
    );
    changed(&set(&folder, ".", "second", true), true);
    assert!(!folder.join("descript.ion").exists());
    assert_eq!(
        fs::read(directory.path().join("descript.ion")).unwrap(),
        b"\xef\xbb\xbf\r\nfolder second\r\n"
    );
}

fn rejected(result: &Output, exit: i32, code: &str) -> serde_json::Value {
    assert_eq!(result.status.code(), Some(exit), "{:?}", result.stderr);
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], code);
    error
}

#[test]
fn rejects_blank_and_reserved_input_without_creating_or_changing_files() {
    let original = "\u{feff}\r\n\"照片 😀.txt\" old\r\n".as_bytes();
    for bytes in [None, Some(original)] {
        let directory = fixture(bytes);
        for body in ["", " \t\u{3000}", "bad\u{4}body"] {
            rejected(
                &set(directory.path(), "照片 😀.txt", body, true),
                2,
                "invalid_argument",
            );
            let file = directory.path().join("descript.ion");
            if let Some(bytes) = bytes {
                assert_eq!(fs::read(file).unwrap(), bytes);
            } else {
                assert!(!file.exists());
            }
        }
        assert_eq!(
            fs::read_dir(directory.path()).unwrap().count(),
            if bytes.is_some() { 2 } else { 1 }
        );
    }
}

#[test]
fn multiline_argument_normalizes_breaks_and_roundtrips_tc_encoding() {
    let directory = fixture(None);
    changed(
        &set(
            directory.path(),
            "照片 😀.txt",
            "  中文\\n\\path\r\n\tsecond\r\n\rthird\n\n",
            true,
        ),
        true,
    );
    assert_eq!(fs::read(directory.path().join("descript.ion")).unwrap(),
        "\u{feff}\r\n\"照片 😀.txt\"   中文\\\\n\\\\path\\n\tsecond\\n\\nthird\\n\\n\u{4}\u{c2}\r\n".as_bytes());
    let result = Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory.path())
        .args(["get", "照片 😀.txt", "--json"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        serde_json::json!({"name":"照片 😀.txt", "comment":"  中文\\n\\path\n\tsecond\n\nthird\n\n", "extension":"tc"})
    );
}

#[test]
fn missing_entries_never_create_records() {
    for bytes in [None, Some(b"\xef\xbb\xbfmissing old\n".as_slice())] {
        let directory = fixture(bytes);
        rejected(
            &set(directory.path(), "missing", "new", true),
            1,
            "io_error",
        );
        let file = directory.path().join("descript.ion");
        if let Some(bytes) = bytes {
            assert_eq!(fs::read(file).unwrap(), bytes);
        } else {
            assert!(!file.exists());
        }
    }
}

#[test]
fn validates_all_records_and_rejects_unknown_extensions_even_for_equal_body() {
    let cases = [
        (
            "\u{feff}\"照片 😀.txt\" old\u{4}other\r\n",
            "unknown_extension",
        ),
        ("\"照片 😀.txt\" old\r\n", "invalid_encoding"),
        ("\u{feff}\"照片 😀.txt\" old\n\"unclosed", "invalid_format"),
        (
            "\u{feff}\"照片 😀.txt\" old\nother first\nOTHER second",
            "invalid_format",
        ),
    ];
    for (bytes, code) in cases {
        let directory = fixture(Some(bytes.as_bytes()));
        rejected(&set(directory.path(), "照片 😀.txt", "old", true), 1, code);
        assert_eq!(
            fs::read(directory.path().join("descript.ion")).unwrap(),
            bytes.as_bytes()
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
    let bytes = b"\xef\xbb\xbf\r\nother \xff";
    let directory = fixture(Some(bytes));
    rejected(
        &set(directory.path(), "照片 😀.txt", "new", true),
        1,
        "invalid_encoding",
    );
    assert_eq!(
        fs::read(directory.path().join("descript.ion")).unwrap(),
        bytes
    );
}

#[test]
fn equal_logical_body_keeps_original_encoding_bytes_and_timestamp() {
    for bytes in [
        "\u{feff}\"照片 😀.txt\"   literal\\n  ",
        "\u{feff}\"照片 😀.txt\"   literal\\\\n  \u{4}\u{c2}\n",
    ] {
        let directory = fixture(Some(bytes.as_bytes()));
        let file = directory.path().join("descript.ion");
        let before = fs::metadata(&file).unwrap().modified().unwrap();
        changed(
            &set(directory.path(), "照片 😀.TXT", r"  literal\n  ", true),
            false,
        );
        assert_eq!(fs::read(&file).unwrap(), bytes.as_bytes());
        assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), before);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}

#[test]
fn serialized_length_counts_unicode_name_quotes_space_and_crlf() {
    let directory = fixture(None);
    // Name = 15 UTF-8 bytes; quotes + separator + CRLF = 5. Body budget = 4076.
    let body = "x".repeat(4076);
    changed(&set(directory.path(), "照片 😀.txt", &body, true), true);
    let file = directory.path().join("descript.ion");
    let before = fs::read(&file).unwrap();
    assert_eq!(before.len(), 5 + 4096);
    rejected(
        &set(directory.path(), "照片 😀.txt", &(body + "x"), true),
        1,
        "invalid_format",
    );
    assert_eq!(fs::read(&file).unwrap(), before);
}

#[test]
fn appending_checks_final_record_length_after_adding_separator() {
    for length in [4094, 4095, 4096] {
        let bytes = format!("\u{feff}\r\n\rlast {}", "x".repeat(length - 5));
        let directory = fixture(Some(bytes.as_bytes()));
        let result = set(directory.path(), "照片 😀.txt", "new", true);
        let file = directory.path().join("descript.ion");
        if length == 4094 {
            changed(&result, true);
            assert_eq!(
                fs::read(&file).unwrap(),
                format!("{bytes}\r\n\"照片 😀.txt\" new\r\n").as_bytes()
            );
        } else {
            let error = rejected(&result, 1, "invalid_format");
            assert_eq!(error["error"]["line"], 3);
            assert_eq!(fs::read(&file).unwrap(), bytes.as_bytes());
        }
    }
}

#[cfg(windows)]
fn powershell(file: &Path, script: &str) -> Output {
    let script = format!("$ErrorActionPreference='Stop'; {script}");
    let result = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .env_remove("PSModulePath")
        .env("DION_TEST_FILE", file)
        .env("DION_TEST_EXE", env!("CARGO_BIN_EXE_dion"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result
}

#[cfg(windows)]
#[test]
fn windows_powershell_utf8_pipeline_preserves_unicode_and_added_tail_break() {
    let directory = fixture(None);
    let entry = directory.path().join("照片 😀.txt");
    // Build Unicode within PowerShell to avoid relying on its script-file encoding.
    let result = powershell(
        &entry,
        "$savedOutputEncoding=$OutputEncoding; try { $OutputEncoding=[System.Text.UTF8Encoding]::new($false); $text=([string][char]0x4e2d)+([char]0x6587)+\"`nsecond\"; $text | & $env:DION_TEST_EXE set $env:DION_TEST_FILE --stdin --json; exit $LASTEXITCODE } finally { $OutputEncoding=$savedOutputEncoding }",
    );
    changed(&result, true);
    let result = Command::new(env!("CARGO_BIN_EXE_dion"))
        .args(["get"])
        .arg(entry)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stderr.is_empty());
    assert_eq!(result.stdout, "中文\nsecond\n".as_bytes());
}

#[cfg(windows)]
#[test]
fn replacement_preserves_attributes_creation_time_and_protected_permissions() {
    use std::os::windows::fs::MetadataExt;
    let directory = fixture(Some("\u{feff}\"照片 😀.txt\" old\r\n".as_bytes()));
    let file = directory.path().join("descript.ion");
    powershell(
        &file,
        "$ErrorActionPreference='Stop'; $p=$env:DION_TEST_FILE; $acl=Get-Acl -LiteralPath $p; $acl.SetAccessRuleProtection($true,$true); Set-Acl -LiteralPath $p -AclObject $acl; [IO.File]::SetCreationTimeUtc($p,[datetime]'2001-02-03T04:05:06Z'); [IO.File]::SetAttributes($p,[IO.FileAttributes]8198)",
    );
    let acl_before = powershell(&file, "(Get-Acl -LiteralPath $env:DION_TEST_FILE).Sddl").stdout;
    let before = fs::metadata(&file).unwrap();
    let entry = directory.path().join("照片 😀.txt");
    let entry_attributes = fs::metadata(&entry).unwrap().file_attributes();
    changed(&set(directory.path(), "照片 😀.txt", "updated", true), true);
    let after = fs::metadata(&file).unwrap();
    assert_eq!(after.file_attributes(), before.file_attributes());
    assert_eq!(after.creation_time(), before.creation_time());
    assert_eq!(
        powershell(&file, "(Get-Acl -LiteralPath $env:DION_TEST_FILE).Sddl").stdout,
        acl_before
    );
    assert_eq!(
        fs::metadata(&entry).unwrap().file_attributes(),
        entry_attributes
    );
    assert_eq!(fs::read(&entry).unwrap(), b"entry");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[cfg(windows)]
#[test]
fn readonly_description_refuses_changes_but_accepts_noop() {
    use std::os::windows::fs::MetadataExt;
    let bytes = "\u{feff}\"照片 😀.txt\" old\n".as_bytes();
    let directory = fixture(Some(bytes));
    let file = directory.path().join("descript.ion");
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&file, permissions).unwrap();
    let attributes = fs::metadata(&file).unwrap().file_attributes();
    rejected(
        &set(directory.path(), "照片 😀.txt", "new", true),
        1,
        "io_error",
    );
    changed(&set(directory.path(), "照片 😀.txt", "old", true), false);
    assert_eq!(fs::read(&file).unwrap(), bytes);
    assert_eq!(fs::metadata(&file).unwrap().file_attributes(), attributes);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[cfg(windows)]
#[test]
fn replacement_sharing_failure_retains_complete_temporary_file_and_reports_paths() {
    use std::os::windows::fs::OpenOptionsExt;
    let bytes = "\u{feff}\"照片 😀.txt\" old\n".as_bytes();
    let directory = fixture(Some(bytes));
    let file = directory.path().join("descript.ion");
    let links = tempfile::tempdir().unwrap();
    let alias = links.path().join("linked");
    let result = Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(&alias)
        .arg(directory.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    // Allows reads but denies deletion, so validation succeeds and replacement fails.
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();
    let error = rejected(&set(&alias, "照片 😀.txt", "new", true), 1, "io_error");
    assert_eq!(fs::read(&file).unwrap(), bytes);
    let recovery = fs::read_dir(&alias)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|extension| extension == "tmp"))
        .unwrap();
    assert_eq!(
        fs::read(&recovery).unwrap(),
        "\u{feff}\"照片 😀.txt\" new\r\n".as_bytes()
    );
    let message = error["error"]["message"].as_str().unwrap();
    // Commit reports canonical paths; a junction or Windows short name can spell
    // the same directory differently from the path returned by read_dir.
    let recovery = fs::canonicalize(recovery).unwrap();
    assert!(
        message.contains(recovery.to_str().unwrap()),
        "reported: {message}; discovered: {}",
        recovery.display()
    );
    assert!(message.contains("descript.ion"));
}

#[cfg(windows)]
#[test]
fn exclusive_occupation_refuses_set_without_creating_temporary_files() {
    use std::os::windows::fs::OpenOptionsExt;
    let bytes = "\u{feff}\"照片 😀.txt\" old\n".as_bytes();
    let directory = fixture(Some(bytes));
    let file = directory.path().join("descript.ion");
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    rejected(
        &set(directory.path(), "照片 😀.txt", "new", true),
        1,
        "io_error",
    );
    drop(locked);
    assert_eq!(fs::read(&file).unwrap(), bytes);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}
