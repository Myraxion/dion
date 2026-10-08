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
fn directories_precede_files_in_natural_order_including_orphans() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("dir10")).unwrap();
    fs::create_dir(directory.path().join("dir2")).unwrap();
    fs::write(directory.path().join("File2"), b"").unwrap();
    fs::write(directory.path().join("descript.ion"),
        "\u{feff}file10 ten\r\ndir10 directory ten\r\nFile2 two\r\ndir2 directory two\r\nfile02 zero two\r\nmissing orphan".as_bytes()).unwrap();
    let result = list(directory.path(), &["list", "--json"]);
    assert_eq!(result.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let names: Vec<_> = value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["dir2", "dir10", "file02", "File2", "file10", "missing"]
    );
    let result = list(directory.path(), &["list"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"dir2\\    directory two\ndir10\\   directory ten\nfile02   zero two\nFile2    two\nfile10   ten\nmissing  orphan\n");
    let result = list(directory.path(), &["list", "-l"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"dir2\\\n    directory two\n\ndir10\\\n    directory ten\n\nfile02\n    zero two\n\nFile2\n    two\n\nfile10\n    ten\n\nmissing\n    orphan\n\n");
}

#[test]
fn columns_align_wide_names_and_every_body_line_without_trimming() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("中😀")).unwrap();
    fs::write(directory.path().join("descript.ion"),
        "\u{feff}a  leading\t\\n\\n tail \\n\\n\u{4}\u{c2}\r\n中😀 directory\r\nempty \r\nＡ fullwidth".as_bytes()).unwrap();
    let result = list(directory.path(), &["list"]);
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stderr.is_empty());
    assert_eq!(
        result.stdout,
        concat!(
            "中😀\\  directory\n",
            "a       leading\t\n",
            "       \n",
            "        tail \n",
            "       \n",
            "       \n",
            "Ａ     fullwidth\n",
            "empty  \n"
        )
        .as_bytes()
    );
}

#[test]
fn long_mode_indents_empty_lines_and_adds_record_spacing_and_json_is_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("docs")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}empty \r\ndocs  first\\n\\n last \\n\\n\u{4}\u{c2}".as_bytes(),
    )
    .unwrap();
    for option in ["--long", "-l"] {
        let result = list(directory.path(), &["list", option]);
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stderr.is_empty());
        assert_eq!(
            result.stdout,
            "docs\\\n     first\n    \n     last \n    \n    \n\nempty\n    \n\n".as_bytes()
        );
        let plain_json = list(directory.path(), &["list", "--json"]);
        let long_json = list(directory.path(), &["list", option, "--json"]);
        assert_eq!(long_json.status.code(), Some(0));
        assert_eq!(plain_json.stdout, long_json.stdout);
    }
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
fn long_names_are_not_wrapped_or_truncated() {
    let directory = tempfile::tempdir().unwrap();
    let name = "长".repeat(100);
    fs::write(
        directory.path().join("descript.ion"),
        format!(
            "\u{feff}{name} {}\\nend\u{4}\u{c2}\r\na short",
            "x".repeat(300)
        ),
    )
    .unwrap();
    let result = list(directory.path(), &["list"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        format!(
            "a{}short\n{name}  {}\n{}end\n",
            " ".repeat(201),
            "x".repeat(300),
            " ".repeat(202)
        )
    );
    let result = list(directory.path(), &["list", "-l"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        format!(
            "a\n    short\n\n{name}\n    {}\n    end\n\n",
            "x".repeat(300)
        )
    );
}

#[test]
fn nonordinary_names_remain_whole_orphans_and_equal_comparisons_use_utf16() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("child")).unwrap();
    fs::create_dir(directory.path().join("child/nested")).unwrap();
    fs::write(
        directory.path().join("child/descript.ion"),
        b"invalid encoding",
    )
    .unwrap();
    // Embedded NUL terminates the Windows comparison, so these distinct legacy
    // names require the specified UTF-16 tiebreaker (they are not case duplicates).
    let names = [
        "same\0b",
        "same\0a",
        "child\\nested",
        "child/nested",
        "..",
        ".",
        "C:\\",
        "child\\",
        "child",
    ];
    let contents = format!(
        "\u{feff}{}",
        names
            .iter()
            .map(|name| format!("\"{name}\" body\r\n"))
            .collect::<String>()
    );
    fs::write(directory.path().join("descript.ion"), contents).unwrap();
    let result = list(directory.path(), &["list", "--json"]);
    assert_eq!(result.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let actual: Vec<_> = value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(actual.len(), names.len());
    assert_eq!(actual[0], "child");
    for name in names {
        assert!(actual.contains(&name));
    }
    let a = actual.iter().position(|name| *name == "same\0a").unwrap();
    let b = actual.iter().position(|name| *name == "same\0b").unwrap();
    assert_eq!(b, a + 1);
    let result = list(directory.path(), &["list", "-l"]);
    assert_eq!(result.status.code(), Some(0));
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.starts_with("child\\\n    body\n\n"));
    for name in names.into_iter().filter(|name| *name != "child") {
        assert!(text.contains(&format!("{name}\n    body\n\n")));
    }
}

#[cfg(windows)]
#[test]
fn all_list_modes_preserve_bytes_times_attributes_and_directory_contents() {
    use std::os::windows::fs::MetadataExt;
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("descript.ion");
    let bytes = b"\xef\xbb\xbf\r\nz last\ra first\nempty ";
    fs::write(&file, bytes).unwrap();
    let original_permissions = fs::metadata(&file).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&file, permissions).unwrap();
    let before = fs::metadata(&file).unwrap();
    for args in [
        vec!["list"],
        vec!["list", "--long"],
        vec!["list", "-l"],
        vec!["list", "--json"],
        vec!["list", "-l", "--json"],
    ] {
        let result = list(directory.path(), &args);
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(fs::read(&file).unwrap(), bytes);
        let after = fs::metadata(&file).unwrap();
        assert_eq!(after.creation_time(), before.creation_time());
        assert_eq!(after.last_write_time(), before.last_write_time());
        assert_eq!(after.file_attributes(), before.file_attributes());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
    // Allow the temporary directory to clean up its read-only fixture.
    fs::set_permissions(&file, original_permissions).unwrap();
}

#[test]
fn default_directory_lists_all_records_in_natural_order_with_decoded_bodies() {
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
            {"name": "empty", "comment": "", "extension": "none"},
            {"name": "orphan", "comment": "normal\\n", "extension": "unknown"},
            {"name": "zeta", "comment": "literal\\n", "extension": "none"},
            {"name": "照片 😀.txt", "comment": "first\n  second\n", "extension": "tc"}
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
            "empty    \nother    C:\\new\n原名 😀  first\n           second\n         \n         \n".as_bytes()
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
        for args in [
            vec!["list"],
            vec!["list", "--long"],
            vec!["list", "--json"],
            vec!["list", "-l", "--json"],
        ] {
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
        vec!["list", "--long", "-l", "--json"],
        vec!["get", "a", "--long", "--json"],
        vec!["set", "a", "body", "-l", "--json"],
        vec!["remove", "a", "--long", "--json"],
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
