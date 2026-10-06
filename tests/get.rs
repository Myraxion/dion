use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn get(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn reads_quoted_unicode_names_and_all_physical_line_endings() {
    let directory = fixture("\u{feff}\r\n\r\n\"照片 😀.txt\" \t中文  \rplain 无扩展\n.hidden 点文件\r\n\"quoted.txt\" C:\\new\\file".as_bytes());
    for (name, comment) in [
        ("照片 😀.txt", "\t中文  "),
        ("plain", "无扩展"),
        (".hidden", "点文件"),
        ("QUOTED.TXT", "C:\\new\\file"),
    ] {
        let result = get(directory.path(), &["get", name]);
        assert_eq!(result.status.code(), Some(0), "{name}: {:?}", result.stderr);
        assert_eq!(result.stdout, comment.as_bytes());
        assert!(result.stderr.is_empty());
    }
}

fn fixture(contents: &[u8]) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("descript.ion"), contents).unwrap();
    directory
}

#[test]
fn reads_an_orphan_comment_without_adding_a_newline() {
    let bytes = b"\xef\xbb\xbfReport.txt   keep whitespace  \r\n";
    let directory = fixture(bytes);
    let result = get(directory.path(), &["get", "report.TXT"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"  keep whitespace  ");
    assert!(result.stderr.is_empty());
    assert_eq!(
        fs::read(directory.path().join("descript.ion")).unwrap(),
        bytes
    );
}

#[test]
fn json_preserves_record_spelling_and_uses_utf8_without_bom_and_final_lf() {
    let directory = fixture("\u{feff}Résumé.txt 中文\t\"引用\" ".as_bytes());
    let result = get(directory.path(), &["get", "RÉSUMÉ.TXT", "--json"]);
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stderr.is_empty());
    assert!(!result.stdout.starts_with(b"\xef\xbb\xbf"));
    assert!(result.stdout.ends_with(b"\n"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        serde_json::json!({"name": "Résumé.txt", "comment": "中文\t\"引用\" ", "extension": "none"})
    );
}

#[test]
fn distinguishes_empty_records_missing_records_and_argument_errors() {
    let directory = fixture(b"\xef\xbb\xbfempty \r\n\"quoted empty\"\nname_only");
    for name in ["empty", "quoted empty", "name_only"] {
        let result = get(directory.path(), &["get", name]);
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }
    for args in [
        vec!["get", "missing", "--json"],
        vec!["--json", "get"],
        vec!["--json", "list"],
        vec!["get", "empty", "extra", "--json"],
    ] {
        let result = get(directory.path(), &args);
        let missing = args.contains(&"missing");
        assert_eq!(result.status.code(), Some(if missing { 3 } else { 2 }));
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(
            error["error"]["code"],
            if missing {
                "not_found"
            } else {
                "invalid_argument"
            }
        );
        assert!(error["error"]["message"].is_string());
    }
    fs::remove_file(directory.path().join("descript.ion")).unwrap();
    let result = get(directory.path(), &["get", "empty", "--json"]);
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["error"]["code"],
        "not_found"
    );
}

#[test]
fn validates_the_whole_file_before_output_and_reports_physical_lines() {
    let cases: Vec<(Vec<u8>, &str, Option<usize>)> = vec![
        (b"target ok\n".to_vec(), "invalid_encoding", None),
        (
            b"\xef\xbb\xbf\r\ntarget ok\r\nother \xff".to_vec(),
            "invalid_encoding",
            Some(3),
        ),
        (
            b"\xef\xbb\xbf\rtarget ok\r\"unclosed note".to_vec(),
            "invalid_format",
            Some(3),
        ),
        (
            b"\xef\xbb\xbftarget ok\nother first\nOTHER second".to_vec(),
            "invalid_format",
            Some(3),
        ),
        (
            "\u{feff}target ok\nÉcole first\néCOLE duplicate"
                .as_bytes()
                .to_vec(),
            "invalid_format",
            Some(3),
        ),
        (
            b"\xef\xbb\xbftarget ok\nother first\nother second".to_vec(),
            "invalid_format",
            Some(3),
        ),
        (
            b"\xef\xbb\xbftarget ok\n\"name\"bad".to_vec(),
            "invalid_format",
            Some(2),
        ),
    ];
    for (bytes, code, line) in cases {
        let directory = fixture(&bytes);
        let result = get(directory.path(), &["get", "target", "--json"]);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert!(result.stderr.ends_with(b"\n"));
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["error"]["code"], code);
        assert_eq!(error["error"]["file"], "descript.ion");
        assert_eq!(
            error["error"]["line"].as_u64(),
            line.map(|value| value as u64)
        );
        assert_eq!(
            fs::read(directory.path().join("descript.ion")).unwrap(),
            bytes
        );
    }
}

#[test]
fn counts_utf8_name_quotes_body_and_actual_terminator_in_4096_byte_limit() {
    for ending in ["", "\r\n", "\n", "\r"] {
        // UTF-8 name: 7 bytes, quotes: 2, separator: 1, body + terminator: 4086.
        let body = "x".repeat(4086 - ending.len());
        let bytes = format!("\u{feff}\"中😀\" {body}{ending}").into_bytes();
        let directory = fixture(&bytes);
        let result = get(directory.path(), &["get", "中😀"]);
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout.len(), 4086 - ending.len());
        let mut too_long = bytes.clone();
        too_long.insert(13, b'x');
        fs::write(directory.path().join("descript.ion"), &too_long).unwrap();
        let result = get(directory.path(), &["get", "中😀", "--json"]);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_format");
        assert_eq!(error["error"]["line"], 1);
        assert_eq!(
            fs::read(directory.path().join("descript.ion")).unwrap(),
            too_long
        );
    }
}

#[test]
fn rejects_unknown_options_and_supports_paths_after_double_dash() {
    let directory =
        fixture(b"\xef\xbb\xbf--json literal option name\n--wat literal unknown option");
    for args in [
        vec!["get", "--wat"],
        vec!["get", "--json", "--json", "--wat"],
    ] {
        let result = get(directory.path(), &args);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
    }
    let result = get(directory.path(), &["get", "--", "--json"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"literal option name");
    let result = get(directory.path(), &["--json", "get", "--", "--wat"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["comment"],
        "literal unknown option"
    );
}

#[test]
fn folder_comment_comes_from_its_parent_for_relative_and_absolute_paths() {
    let directory = fixture(b"\xef\xbb\xbffolder parent comment");
    let folder = directory.path().join("folder");
    fs::create_dir(&folder).unwrap();
    fs::write(
        folder.join("descript.ion"),
        b"\xef\xbb\xbffolder wrong comment",
    )
    .unwrap();
    for path in ["./folder", "folder/", folder.to_str().unwrap()] {
        let result = get(directory.path(), &["get", path]);
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"parent comment");
    }
}

#[test]
fn inaccessible_parent_is_an_operation_error_instead_of_a_missing_comment() {
    let directory = tempfile::tempdir().unwrap();
    let result = get(
        directory.path(),
        &["get", "absent-parent/file.txt", "--json"],
    );
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], "io_error");
    assert!(error["error"].get("line").is_none());
    fs::write(directory.path().join("parent-file"), b"file").unwrap();
    let result = get(directory.path(), &["get", "parent-file/child", "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
}

#[test]
fn bom_only_and_header_blank_lines_have_no_records() {
    for bytes in [
        b"\xef\xbb\xbf".as_slice(),
        b"\xef\xbb\xbf\r\n\n\r\r\n".as_slice(),
    ] {
        let directory = fixture(bytes);
        let result = get(directory.path(), &["get", "anything"]);
        assert_eq!(result.status.code(), Some(3));
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
}

#[cfg(windows)]
#[test]
fn reads_readonly_comments_without_changing_bytes_or_entry_attributes() {
    use std::os::windows::fs::MetadataExt;
    let bytes = b"\xef\xbb\xbftarget read only\nfolder folder comment";
    let directory = fixture(bytes);
    let description = directory.path().join("descript.ion");
    let target = directory.path().join("target");
    let folder = directory.path().join("folder");
    fs::write(&target, b"target contents").unwrap();
    fs::create_dir(&folder).unwrap();
    let mut permissions = fs::metadata(&description).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&description, permissions).unwrap();
    let before: Vec<_> = [&description, &target, &folder]
        .iter()
        .map(|path| fs::metadata(path).unwrap().file_attributes())
        .collect();
    for name in ["target", "folder"] {
        let result = get(directory.path(), &["get", name]);
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stderr.is_empty());
    }
    let after: Vec<_> = [&description, &target, &folder]
        .iter()
        .map(|path| fs::metadata(path).unwrap().file_attributes())
        .collect();
    assert_eq!(before, after);
    assert_eq!(fs::read(&description).unwrap(), bytes);
    assert_eq!(fs::read(&target).unwrap(), b"target contents");
}

#[cfg(windows)]
#[test]
fn a_sharing_violation_is_an_io_error() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = fixture(b"\xef\xbb\xbftarget ok");
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(directory.path().join("descript.ion"))
        .unwrap();
    let result = get(directory.path(), &["get", "target", "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], "io_error");
    assert_eq!(error["error"]["file"], "descript.ion");
}

#[test]
fn program_extensions_are_explicitly_deferred_instead_of_reported_as_none() {
    let directory = fixture(b"\xef\xbb\xbftarget first\\nsecond\x04\xc3\x82");
    let result = get(directory.path(), &["get", "target", "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], "unsupported_extension");
    assert_eq!(error["error"]["line"], 1);
}
