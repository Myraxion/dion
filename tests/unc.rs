#![cfg(windows)]

use std::{
    fs,
    io::Write,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

// Explicit opt-in: a local directory is never a substitute for an SMB share.
fn share() -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os("DION_UNC_ROOT")
            .expect("set DION_UNC_ROOT to a writable UNC test directory"),
    );
    assert!(
        matches!(path.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::UNC(..) | std::path::Prefix::VerbatimUNC(..))),
        "DION_UNC_ROOT must be UNC"
    );
    path
}

fn fixture() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("dion-unc-")
        .tempdir_in(share())
        .unwrap()
}

fn run(args: &[&str], stdin: Option<&[u8]>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_dion"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(bytes) = stdin {
        child.stdin.take().unwrap().write_all(bytes).unwrap();
    } else {
        drop(child.stdin.take());
    }
    child.wait_with_output().unwrap()
}

fn success(result: &Output) {
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
}

fn rejected(result: &Output, exit: i32, code: &str) -> serde_json::Value {
    assert_eq!(result.status.code(), Some(exit), "{:?}", result.stderr);
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], code);
    error
}

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

#[test]
#[ignore = "requires a real writable SMB share in DION_UNC_ROOT"]
fn unc_sources_roundtrip_and_preserve_metadata_on_long_unicode_paths() {
    let directory = fixture();
    let folder =
        directory
            .path()
            .join(format!("{}\\{}\\资料 😀", "a".repeat(120), "b".repeat(120)));
    fs::create_dir_all(&folder).unwrap();
    let extended = fs::canonicalize(&folder).unwrap();
    assert!(
        matches!(extended.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::VerbatimUNC(..)))
    );
    assert!(folder.as_os_str().len() > 260);
    let entry = folder.join("照片 😀.txt");
    fs::write(&entry, b"entry").unwrap();
    let attributes = fs::metadata(&entry).unwrap().file_attributes();
    let file = folder.join("descript.ion");
    for path in [&folder, &extended] {
        let target = path.join("照片 😀.txt");
        let target = target.to_str().unwrap();
        for source in ["argument", "--stdin", "--comment-file"] {
            let body = "  中文\\n\\path\r\n\tsecond\rthird\n\n";
            let input = path.join("正文.txt");
            fs::write(&input, format!("\u{feff}{body}")).unwrap();
            let result = match source {
                "argument" => run(&["--json", "set", target, body], None),
                "--stdin" => run(
                    &["--json", "set", target, source],
                    Some(format!("\u{feff}{body}").as_bytes()),
                ),
                _ => run(
                    &["--json", "set", target, source, input.to_str().unwrap()],
                    None,
                ),
            };
            success(&result);
            assert_eq!(result.stdout, b"{\"changed\":true}\n");
            assert_eq!(fs::read(&file).unwrap(), "\u{feff}\r\n\"照片 😀.txt\"   中文\\\\n\\\\path\\n\tsecond\\nthird\\n\\n\u{4}\u{c2}\r\n".as_bytes());
            assert_ne!(fs::metadata(&file).unwrap().file_attributes() & 2, 0);
            let normalized = "  中文\\n\\path\n\tsecond\nthird\n\n";
            let result = run(&["get", target], None);
            success(&result);
            assert_eq!(result.stdout, normalized.as_bytes());
            let record =
                serde_json::json!({"name":"照片 😀.txt", "comment":normalized, "extension":"tc"});
            let result = run(&["--json", "get", target], None);
            success(&result);
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
                record
            );
            let result = run(&["--json", "list", path.to_str().unwrap()], None);
            success(&result);
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
                serde_json::json!({"entries":[record]})
            );
            let result = run(&["list", path.to_str().unwrap()], None);
            success(&result);
            assert_eq!(
                result.stdout,
                format!("照片 😀.txt:\n{normalized}\n").as_bytes()
            );

            powershell(
                &file,
                "$p=$env:DION_TEST_FILE; $acl=Get-Acl -LiteralPath $p; $acl.SetAccessRuleProtection($true,$true); Set-Acl -LiteralPath $p -AclObject $acl; [IO.File]::SetCreationTimeUtc($p,[datetime]'2001-02-03T04:05:06Z'); [IO.File]::SetAttributes($p,[IO.FileAttributes]8198)",
            );
            let before = fs::metadata(&file).unwrap();
            let acl = powershell(&file, "(Get-Acl -LiteralPath $env:DION_TEST_FILE).Sddl").stdout;
            let result = run(&["set", target, "replacement"], None);
            success(&result);
            assert!(result.stdout.is_empty());
            assert_eq!(
                fs::metadata(&file).unwrap().file_attributes(),
                before.file_attributes()
            );
            assert_eq!(
                fs::metadata(&file).unwrap().creation_time(),
                before.creation_time()
            );
            assert_eq!(
                powershell(&file, "(Get-Acl -LiteralPath $env:DION_TEST_FILE).Sddl").stdout,
                acl
            );
            assert_eq!(fs::metadata(&entry).unwrap().file_attributes(), attributes);
            assert_eq!(fs::read(&entry).unwrap(), b"entry");
            let result = run(&["--json", "remove", target], None);
            success(&result);
            assert_eq!(result.stdout, b"{\"changed\":true}\n");
            assert!(!file.exists());
            let result = run(&["--json", "remove", target], None);
            success(&result);
            assert_eq!(result.stdout, b"{\"changed\":false}\n");
            rejected(&run(&["--json", "get", target], None), 3, "not_found");
            let result = run(&["--json", "list", path.to_str().unwrap()], None);
            success(&result);
            assert_eq!(result.stdout, b"{\"entries\":[]}\n");
            assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
        }
    }
}

#[test]
#[ignore = "requires a real SMB share in DION_UNC_ROOT"]
fn unc_share_root_lists_but_has_no_entry_comment() {
    let root = share().ancestors().last().unwrap().to_owned();
    let extended = fs::canonicalize(&root).unwrap();
    for path in [&root, &extended] {
        let path = path.to_str().unwrap();
        let result = run(&["--json", "list", path], None);
        success(&result);
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert!(value["entries"].is_array());
        for command in ["get", "set", "remove"] {
            let mut args = vec!["--json", command, path];
            if command == "set" {
                args.push("body");
            }
            rejected(&run(&args, None), 2, "invalid_argument");
        }
    }
}

#[test]
#[ignore = "requires a real writable SMB share in DION_UNC_ROOT"]
fn unc_occupation_and_readonly_are_errors_and_replacement_retains_recovery() {
    let directory = fixture();
    let entry = directory.path().join("entry");
    fs::write(&entry, b"entry").unwrap();
    let target = entry.to_str().unwrap();
    let file = directory.path().join("descript.ion");
    let original = b"\xef\xbb\xbfentry old\nother untouched\n";
    fs::write(&file, original).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    for command in ["get", "list", "set", "remove"] {
        let path = if command == "list" {
            directory.path().to_str().unwrap()
        } else {
            target
        };
        let mut args = vec!["--json", command, path];
        if command == "set" {
            args.push("new");
        }
        rejected(&run(&args, None), 1, "io_error");
    }
    drop(lock);
    assert_eq!(fs::read(&file).unwrap(), original);
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();
    for command in ["set", "remove"] {
        let mut args = vec!["--json", command, target];
        if command == "set" {
            args.push("new");
        }
        let error = rejected(&run(&args, None), 1, "io_error");
        assert_eq!(fs::read(&file).unwrap(), original);
        let recovery: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "tmp"))
            .collect();
        assert_eq!(recovery.len(), 1);
        assert_eq!(recovery[0].parent(), Some(directory.path()));
        let expected: &[u8] = if command == "set" {
            b"\xef\xbb\xbfentry new\r\nother untouched\n"
        } else {
            b"\xef\xbb\xbfother untouched\n"
        };
        assert_eq!(fs::read(&recovery[0]).unwrap(), expected);
        let message = error["error"]["message"].as_str().unwrap();
        assert!(message.contains(fs::canonicalize(&recovery[0]).unwrap().to_str().unwrap()));
        assert!(message.contains(fs::canonicalize(&file).unwrap().to_str().unwrap()));
        fs::remove_file(&recovery[0]).unwrap();
    }
    drop(lock);
    fs::write(&file, b"\xef\xbb\xbfentry old\n").unwrap();
    powershell(
        &file,
        "[IO.File]::SetAttributes($env:DION_TEST_FILE,[IO.FileAttributes]3)",
    );
    for command in ["set", "remove"] {
        let mut args = vec!["--json", command, target];
        if command == "set" {
            args.push("new");
        }
        rejected(&run(&args, None), 1, "io_error");
        assert_eq!(fs::read(&file).unwrap(), b"\xef\xbb\xbfentry old\n");
    }
    success(&run(&["get", target], None));
    powershell(
        &file,
        "[IO.File]::SetAttributes($env:DION_TEST_FILE,[IO.FileAttributes]2)",
    );
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();
    rejected(&run(&["--json", "remove", target], None), 1, "io_error");
    drop(lock);
    assert_eq!(fs::read(&file).unwrap(), b"\xef\xbb\xbfentry old\n");
}

#[test]
#[ignore = "requires a real SMB server in DION_UNC_ROOT"]
fn unavailable_share_is_an_operation_error_for_every_command() {
    let share = share();
    assert!(fs::metadata(&share).unwrap().is_dir());
    let server = match share.components().next().unwrap() {
        std::path::Component::Prefix(prefix) => match prefix.kind() {
            std::path::Prefix::UNC(server, _) | std::path::Prefix::VerbatimUNC(server, _) => {
                server.to_owned()
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    let missing = format!(
        "\\\\{}\\dion-missing-{}",
        server.to_str().unwrap(),
        std::process::id()
    );
    for folder in [missing.clone(), format!("\\\\?\\UNC\\{}", &missing[2..])] {
        for command in ["get", "list", "set", "remove"] {
            let entry = format!("{folder}\\entry");
            let path = if command == "list" { &folder } else { &entry };
            let mut args = vec!["--json", command, path];
            if command == "set" {
                args.push("body");
            }
            rejected(&run(&args, None), 1, "io_error");
        }
    }
}

#[test]
#[ignore = "requires an SMB share with Windows ACL support in DION_UNC_ROOT"]
fn unc_denied_description_access_is_never_reported_as_absent_or_success() {
    let directory = fixture();
    let entry = directory.path().join("entry");
    fs::write(&entry, b"entry").unwrap();
    let file = directory.path().join("descript.ion");
    let original = b"\xef\xbb\xbfentry old\n";
    fs::write(&file, original).unwrap();
    powershell(
        &file,
        "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value; & icacls.exe $env:DION_TEST_FILE /deny ('*'+$sid+':(RD)'); if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
    );
    // Restore access before assertions so a failing product check does not leave
    // a denied file behind. Only this fixture's ACL is modified.
    let results: Vec<_> = ["get", "list", "set", "remove"]
        .into_iter()
        .map(|command| {
            let path = if command == "list" {
                directory.path()
            } else {
                &entry
            };
            let mut args = vec!["--json", command, path.to_str().unwrap()];
            if command == "set" {
                args.push("new");
            }
            run(&args, None)
        })
        .collect();
    powershell(
        &file,
        "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value; & icacls.exe $env:DION_TEST_FILE /remove:d ('*'+$sid); if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
    );
    for result in results {
        rejected(&result, 1, "io_error");
    }
    assert_eq!(fs::read(&file).unwrap(), original);
}
