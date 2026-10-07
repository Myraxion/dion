use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn list(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn an_existing_zero_byte_file_is_an_encoding_error_not_an_empty_list() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("descript.ion"), b"").unwrap();
    let result = list(directory.path(), &["list", "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["error"]["code"],
        "invalid_encoding"
    );
}

#[test]
fn default_directory_lists_all_records_in_original_order_with_decoded_bodies() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = "\u{feff}\r\n\"照片 😀.txt\" first\\n  second\\n\u{4}\u{c2}\r\nzeta literal\\n\nempty \rorphan normal\\n\u{4}foreign".as_bytes();
    fs::write(directory.path().join("descript.ion"), bytes).unwrap();
    let result = list(directory.path(), &["list", "--json"]);
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stderr.is_empty());
    assert!(!result.stdout.starts_with(b"\xef\xbb\xbf"));
    assert!(result.stdout.ends_with(b"\n"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        serde_json::json!({"entries": [
            {"name": "照片 😀.txt", "comment": "first\n  second\n", "extension": "tc"},
            {"name": "zeta", "comment": "literal\\n", "extension": "none"},
            {"name": "empty", "comment": "", "extension": "none"},
            {"name": "orphan", "comment": "normal\\n", "extension": "unknown"}
        ]})
    );
    assert_eq!(
        fs::read(directory.path().join("descript.ion")).unwrap(),
        bytes
    );
}

#[test]
fn explicit_relative_and_absolute_directories_show_complete_text_without_recursing() {
    let directory = tempfile::tempdir().unwrap();
    let child = directory.path().join("备注目录");
    fs::create_dir(&child).unwrap();
    fs::create_dir(child.join("nested")).unwrap();
    fs::write(
        child.join("nested/descript.ion"),
        b"\xef\xbb\xbfhidden nested",
    )
    .unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfwrong parent",
    )
    .unwrap();
    fs::write(child.join("descript.ion"), "\u{feff}\"原名 😀\" first\\n  second\\n\\n\u{4}\u{c2}\r\nempty \nother C:\\new\u{4}foreign".as_bytes()).unwrap();
    for path in ["备注目录", child.to_str().unwrap()] {
        let result = list(directory.path(), &["list", path]);
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stderr.is_empty());
        assert_eq!(
            result.stdout,
            "原名 😀:\nfirst\n  second\n\n\nempty:\n\nother:\nC:\\new\n".as_bytes()
        );
    }
}

#[test]
fn missing_description_and_header_only_files_are_successful_empty_lists() {
    let directory = tempfile::tempdir().unwrap();
    for contents in [
        None,
        Some(b"\xef\xbb\xbf".as_slice()),
        Some(b"\xef\xbb\xbf\r\n\n\r\r\n".as_slice()),
    ] {
        if let Some(bytes) = contents {
            fs::write(directory.path().join("descript.ion"), bytes).unwrap();
        }
        let result = list(directory.path(), &["--json", "list", "."]);
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"{\"entries\":[]}\n");
        assert!(result.stderr.is_empty());
        let result = list(directory.path(), &["list"]);
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn late_bad_records_fail_the_whole_list_in_text_and_json() {
    let directory = tempfile::tempdir().unwrap();
    for (bad, code) in [
        (b"bad \xff".to_vec(), "invalid_encoding"),
        (b"\"unclosed note".to_vec(), "invalid_format"),
        (b"FIRST duplicate".to_vec(), "invalid_format"),
        (
            format!("long {}", "x".repeat(4096)).into_bytes(),
            "invalid_format",
        ),
    ] {
        let mut bytes =
            b"\xef\xbb\xbf\r\nfirst ok\r\nsecond multiline\\nbody\x04\xc3\x82\n".to_vec();
        bytes.extend(bad);
        fs::write(directory.path().join("descript.ion"), &bytes).unwrap();
        for args in [vec!["list"], vec!["list", "--json"]] {
            let result = list(directory.path(), &args);
            assert_eq!(result.status.code(), Some(1));
            assert!(result.stdout.is_empty());
            if args.contains(&"--json") {
                let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
                assert_eq!(error["error"]["code"], code);
                assert_eq!(error["error"]["file"], ".\\descript.ion");
                assert_eq!(error["error"]["line"], 4);
            } else {
                let error = String::from_utf8(result.stderr).unwrap();
                assert!(error.contains(code));
                assert!(error.contains("descript.ion"));
                assert!(error.contains("line 4"));
            }
        }
        assert_eq!(
            fs::read(directory.path().join("descript.ion")).unwrap(),
            bytes
        );
    }
}

#[test]
fn inaccessible_directories_and_description_files_are_operation_errors() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("file"), b"not a directory").unwrap();
    fs::create_dir(directory.path().join("descript.ion")).unwrap();
    for path in ["missing", "file", "."] {
        let result = list(directory.path(), &["list", path, "--json"]);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["error"]["code"],
            "io_error"
        );
    }
}

#[test]
fn list_rejects_extra_arguments_and_options_and_supports_directory_after_separator() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("--json")).unwrap();
    for args in [
        vec!["list", ".", "extra", "--json"],
        vec!["list", "--wat", "--json"],
        vec!["list", "--json", "--json"],
    ] {
        let result = list(directory.path(), &args);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["error"]["code"],
            "invalid_argument"
        );
    }
    let result = list(directory.path(), &["--json", "list", "--", "--json"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"{\"entries\":[]}\n");
    assert!(result.stderr.is_empty());
}

#[cfg(windows)]
#[test]
fn locked_description_is_an_operation_error() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbffirst ok",
    )
    .unwrap();
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(directory.path().join("descript.ion"))
        .unwrap();
    let result = list(directory.path(), &["list", "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["error"]["code"],
        "io_error"
    );
}
