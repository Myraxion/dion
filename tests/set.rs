use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn set(directory: &Path, name: &str, body: &str, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
    command.current_dir(directory);
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
fn rejects_blank_reserved_and_multiline_input_without_creating_or_changing_files() {
    let original = "\u{feff}\r\n\"照片 😀.txt\" old\r\n".as_bytes();
    for bytes in [None, Some(original)] {
        let directory = fixture(bytes);
        for body in [
            "",
            " \t\u{3000}",
            "bad\u{4}body",
            "first\nsecond",
            "first\rsecond",
        ] {
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
    // Allows reads but denies deletion, so validation succeeds and replacement fails.
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();
    let error = rejected(
        &set(directory.path(), "照片 😀.txt", "new", true),
        1,
        "io_error",
    );
    assert_eq!(fs::read(&file).unwrap(), bytes);
    let recovery = fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|extension| extension == "tmp"))
        .unwrap();
    assert_eq!(
        fs::read(&recovery).unwrap(),
        "\u{feff}\"照片 😀.txt\" new\r\n".as_bytes()
    );
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains(recovery.to_str().unwrap()));
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
