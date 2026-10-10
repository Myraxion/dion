use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn remove(directory: &Path, name: &str, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
    command.current_dir(directory).env("DION_LANG", "en");
    if json {
        command.arg("--json");
    }
    command.args(["remove", "--", name]).output().unwrap()
}

fn changed(result: &Output, expected: bool) {
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    assert!(result.stderr.is_empty());
    assert_eq!(
        result.stdout,
        format!("{{\"changed\":{expected}}}\n").as_bytes()
    );
}

#[test]
fn removes_orphan_case_insensitively_and_preserves_other_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("descript.ion");
    fs::write(
        &file,
        "\u{feff}\r\nforeign raw\\n\u{4}unknown\r\"照片 😀.TXT\" old\n\r\nlast   tail",
    )
    .unwrap();
    changed(&remove(directory.path(), "照片 😀.txt", true), true);
    assert_eq!(
        fs::read(&file).unwrap(),
        "\u{feff}\r\nforeign raw\\n\u{4}unknown\r\r\nlast   tail".as_bytes()
    );
    let timestamp = fs::metadata(&file).unwrap().modified().unwrap();
    changed(&remove(directory.path(), "照片 😀.txt", true), false);
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), timestamp);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn deleting_last_record_removes_file_including_header_and_blank_lines() {
    for record in ["orphan old", "orphan", "orphan body\u{4}unknown"] {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("descript.ion");
        fs::write(&file, format!("\u{feff}\r\n\n{record}\r\n\r\n")).unwrap();
        let result = remove(directory.path(), "orphan", false);
        assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        assert!(!file.exists());
        changed(&remove(directory.path(), "orphan", true), false);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn remaining_empty_and_unknown_records_count_and_allow_explicit_deletion() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("descript.ion");
    fs::write(
        &file,
        b"\xef\xbb\xbftarget body\r\nempty\nunknown raw\x04foreign",
    )
    .unwrap();
    changed(&remove(directory.path(), "target", true), true);
    assert_eq!(
        fs::read(&file).unwrap(),
        b"\xef\xbb\xbfempty\nunknown raw\x04foreign"
    );
    changed(&remove(directory.path(), "unknown", true), true);
    assert_eq!(fs::read(&file).unwrap(), b"\xef\xbb\xbfempty\n");
    changed(&remove(directory.path(), "empty", true), true);
    assert!(!file.exists());
}

#[test]
fn missing_record_never_creates_or_cleans_description_files() {
    for bytes in [
        None,
        Some(b"\xef\xbb\xbf".as_slice()),
        Some(b"\xef\xbb\xbf\r\n\n\r\n"),
        Some(b"\xef\xbb\xbfother body\n"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("descript.ion");
        if let Some(bytes) = bytes {
            fs::write(&file, bytes).unwrap();
        }
        let timestamp = bytes.map(|_| fs::metadata(&file).unwrap().modified().unwrap());
        changed(&remove(directory.path(), "missing", true), false);
        if let Some(bytes) = bytes {
            assert_eq!(fs::read(&file).unwrap(), bytes);
            assert_eq!(
                fs::metadata(&file).unwrap().modified().unwrap(),
                timestamp.unwrap()
            );
        } else {
            assert!(!file.exists());
        }
        assert_eq!(
            fs::read_dir(directory.path()).unwrap().count(),
            usize::from(bytes.is_some())
        );
    }
}

fn rejected(result: &Output, exit: i32, code: &str) -> serde_json::Value {
    assert_eq!(result.status.code(), Some(exit), "{:?}", result.stderr);
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], code);
    error
}

#[test]
fn validates_entire_file_even_when_target_is_missing() {
    for (bytes, code, line) in [
        (b"target body\n".to_vec(), "invalid_encoding", None),
        (
            b"\xef\xbb\xbf\r\ntarget body\n\"unclosed".to_vec(),
            "invalid_format",
            Some(3),
        ),
        (
            b"\xef\xbb\xbftarget body\nother first\nOTHER second".to_vec(),
            "invalid_format",
            Some(3),
        ),
        (
            b"\xef\xbb\xbftarget body\nother \xff".to_vec(),
            "invalid_encoding",
            Some(2),
        ),
        (
            format!("\u{feff}target body\nother {}", "x".repeat(4091)).into_bytes(),
            "invalid_format",
            Some(2),
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("descript.ion");
        fs::write(&file, &bytes).unwrap();
        for target in ["target", "missing"] {
            let error = rejected(&remove(directory.path(), target, true), 1, code);
            assert_eq!(error["error"]["file"], file.to_str().unwrap());
            if let Some(line) = line {
                assert_eq!(error["error"]["line"], line);
            }
            assert_eq!(fs::read(&file).unwrap(), bytes);
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn invalid_arguments_and_inaccessible_parent_are_errors() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec!["remove"],
        vec!["remove", "a", "b"],
        vec!["remove", "a", "--stdin"],
        vec!["remove", "a", "--comment-file", "body"],
        vec!["remove", ""],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_dion"))
            .current_dir(directory.path())
            .arg("--json")
            .args(args)
            .output()
            .unwrap();
        rejected(&result, 2, "invalid_argument");
    }
    rejected(
        &remove(directory.path(), "missing/entry", true),
        1,
        "io_error",
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn readonly_description_refuses_both_replacement_and_deletion_but_accepts_noop() {
    use std::os::windows::fs::MetadataExt;
    for bytes in [
        b"\xef\xbb\xbftarget body\n".as_slice(),
        b"\xef\xbb\xbftarget body\nother body\n",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("descript.ion");
        fs::write(&file, bytes).unwrap();
        let mut permissions = fs::metadata(&file).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&file, permissions).unwrap();
        let attributes = fs::metadata(&file).unwrap().file_attributes();
        rejected(&remove(directory.path(), "target", true), 1, "io_error");
        changed(&remove(directory.path(), "missing", true), false);
        assert_eq!(fs::read(&file).unwrap(), bytes);
        assert_eq!(fs::metadata(&file).unwrap().file_attributes(), attributes);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[cfg(windows)]
#[test]
fn occupied_last_record_file_refuses_deletion_without_leaving_temporary_files() {
    use std::os::windows::fs::OpenOptionsExt;
    for sharing in [0, 1] {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("descript.ion");
        let bytes = b"\xef\xbb\xbftarget body\n";
        fs::write(&file, bytes).unwrap();
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(sharing)
            .open(&file)
            .unwrap();
        rejected(&remove(directory.path(), "target", true), 1, "io_error");
        drop(locked);
        assert_eq!(fs::read(&file).unwrap(), bytes);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[cfg(windows)]
fn powershell(file: &Path, script: &str) -> Output {
    let result = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("$ErrorActionPreference='Stop'; {script}"),
        ])
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
fn partial_removal_preserves_attributes_creation_time_and_protected_permissions() {
    use std::os::windows::fs::MetadataExt;
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("descript.ion");
    let entry = directory.path().join("target");
    fs::write(&entry, b"entry").unwrap();
    fs::write(&file, b"\xef\xbb\xbftarget body\nother body\n").unwrap();
    powershell(
        &file,
        "$p=$env:DION_TEST_FILE; $acl=Get-Acl -LiteralPath $p; $acl.SetAccessRuleProtection($true,$true); Set-Acl -LiteralPath $p -AclObject $acl; [IO.File]::SetCreationTimeUtc($p,[datetime]'2001-02-03T04:05:06Z'); [IO.File]::SetAttributes($p,[IO.FileAttributes]8198)",
    );
    let before = fs::metadata(&file).unwrap();
    let acl = powershell(&file, "(Get-Acl -LiteralPath $env:DION_TEST_FILE).Sddl").stdout;
    let entry_attributes = fs::metadata(&entry).unwrap().file_attributes();
    changed(&remove(directory.path(), "target", true), true);
    let after = fs::metadata(&file).unwrap();
    assert_eq!(after.file_attributes(), before.file_attributes());
    assert_eq!(after.creation_time(), before.creation_time());
    assert_eq!(
        powershell(&file, "(Get-Acl -LiteralPath $env:DION_TEST_FILE).Sddl").stdout,
        acl
    );
    assert_eq!(fs::read(&file).unwrap(), b"\xef\xbb\xbfother body\n");
    assert_eq!(fs::read(&entry).unwrap(), b"entry");
    assert_eq!(
        fs::metadata(&entry).unwrap().file_attributes(),
        entry_attributes
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[cfg(windows)]
#[test]
fn partial_removal_sharing_failure_reports_complete_recovery_file() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("descript.ion");
    let bytes = b"\xef\xbb\xbftarget body\nother body\n";
    fs::write(&file, bytes).unwrap();
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();
    let error = rejected(&remove(directory.path(), "target", true), 1, "io_error");
    assert_eq!(fs::read(&file).unwrap(), bytes);
    let recovery = fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|extension| extension == "tmp"))
        .unwrap();
    assert_eq!(fs::read(&recovery).unwrap(), b"\xef\xbb\xbfother body\n");
    let recovery = fs::canonicalize(recovery).unwrap();
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains(recovery.to_str().unwrap()));
    assert!(message.contains("descript.ion"));
}
