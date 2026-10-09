#![cfg(windows)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};
use tempfile::TempDir;

fn editor() -> &'static Path {
    static EDITOR: OnceLock<(TempDir, PathBuf)> = OnceLock::new();
    &EDITOR
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let exe = dir.path().join("controlled editor.exe");
            let result = Command::new("rustc")
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/editor.rs"))
                .arg("--edition=2024")
                .arg("-o")
                .arg(&exe)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            (dir, exe)
        })
        .1
}

fn fixture(original: Option<&[u8]>) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("照片 😀.txt"), b"entry").unwrap();
    if let Some(bytes) = original {
        fs::write(dir.path().join("descript.ion"), bytes).unwrap();
    }
    dir
}

fn edit(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_dion"));
    cmd.current_dir(dir)
        .args(["set", "照片 😀.txt", "--edit", "--json"])
        .env(
            "VISUAL",
            format!("\"{}\" --label \"quoted value\"", editor().display()),
        )
        .env_remove("EDITOR")
        .env_remove("EDIT_BODY")
        .env_remove("EDIT_GATE")
        .env_remove("EDIT_CONFLICT")
        .env_remove("EDIT_EXPECT_LABEL")
        .env_remove("EDIT_EXIT");
    cmd
}

fn changed(output: &Output, value: bool) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"changed": value})
    );
}

fn edit_path(dir: &Path) -> PathBuf {
    PathBuf::from(fs::read_to_string(dir.join("edit-path.txt")).unwrap())
}

#[test]
fn prefills_logical_body_and_keeps_bytes_and_time_when_unchanged() {
    let original = "\u{feff}\r\n\"照片 😀.txt\"  中文\\n尾\\n\u{4}\u{c2}\r";
    let dir = fixture(Some(original.as_bytes()));
    let file = dir.path().join("descript.ion");
    let time = fs::metadata(&file).unwrap().modified().unwrap();
    changed(&edit(dir.path()).output().unwrap(), false);
    assert_eq!(
        fs::read(dir.path().join("prefill.txt")).unwrap(),
        " 中文\n尾\n".as_bytes()
    );
    assert_eq!(fs::read(&file).unwrap(), original.as_bytes());
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), time);
    assert!(!edit_path(dir.path()).exists());
}

#[test]
fn clearing_removes_records_including_empty_orphan_and_unknown_extension() {
    for (original, target_exists) in [
        ("\u{feff}\"照片 😀.txt\" old\r\nother keep\n", true),
        ("\u{feff}\"照片 😀.txt\"\r\n", true),
        ("\u{feff}\"照片 😀.txt\" old\r\n", false),
        ("\u{feff}\"照片 😀.txt\" old\u{4}opaque\r\n", true),
    ] {
        let dir = fixture(Some(original.as_bytes()));
        if !target_exists {
            fs::remove_file(dir.path().join("照片 😀.txt")).unwrap();
        }
        fs::write(dir.path().join("body.txt"), b"").unwrap();
        changed(
            &edit(dir.path())
                .env("EDIT_BODY", "body.txt")
                .output()
                .unwrap(),
            true,
        );
        let file = dir.path().join("descript.ion");
        if original.contains("other") {
            assert_eq!(fs::read(file).unwrap(), b"\xef\xbb\xbfother keep\n");
        } else {
            assert!(!file.exists());
        }
        assert!(!edit_path(dir.path()).exists());
    }
}

fn rejected(output: &Output, exit: i32, code: &str, _dir: &Path) -> Vec<u8> {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], code);
    let message = error["error"]["message"].as_str().unwrap();
    let path = PathBuf::from(message.split_once("; Edit text retained at ").unwrap().1);
    assert!(path.exists());
    assert!(
        error["error"]
            .as_object()
            .unwrap()
            .keys()
            .all(|key| ["code", "message", "file", "line"].contains(&key.as_str()))
    );
    let bytes = fs::read(&path).unwrap();
    fs::remove_file(path).unwrap();
    bytes
}

#[test]
fn absent_record_prefills_empty_and_does_not_create_or_modify_description() {
    for original in [None, Some(b"\xef\xbb\xbfother keep\r\n".as_slice())] {
        let dir = fixture(original);
        fs::remove_file(dir.path().join("照片 😀.txt")).unwrap();
        changed(&edit(dir.path()).output().unwrap(), false);
        assert!(fs::read(dir.path().join("prefill.txt")).unwrap().is_empty());
        let file = dir.path().join("descript.ion");
        if let Some(original) = original {
            assert_eq!(fs::read(file).unwrap(), original);
        } else {
            assert!(!file.exists());
        }
        assert!(!edit_path(dir.path()).exists());
    }
}

#[test]
fn unchanged_empty_record_is_deleted() {
    let dir = fixture(Some("\u{feff}\"照片 😀.txt\"\r\n".as_bytes()));
    changed(&edit(dir.path()).output().unwrap(), true);
    assert!(fs::read(dir.path().join("prefill.txt")).unwrap().is_empty());
    assert!(!dir.path().join("descript.ion").exists());
    assert!(!edit_path(dir.path()).exists());
}

#[test]
fn saves_utf8_with_optional_bom_and_normalizes_line_breaks_without_trimming() {
    for bom in ["", "\u{feff}"] {
        let original = "\u{feff}\r\nother keep\n\"照片 😀.txt\" old\r";
        let dir = fixture(Some(original.as_bytes()));
        fs::write(
            dir.path().join("body.txt"),
            format!("{bom}  中文\\folder\r\n\tsecond\rtail \r\n\r\n"),
        )
        .unwrap();
        changed(
            &edit(dir.path())
                .env("EDIT_BODY", "body.txt")
                .output()
                .unwrap(),
            true,
        );
        let get = Command::new(env!("CARGO_BIN_EXE_dion"))
            .current_dir(dir.path())
            .args(["get", "照片 😀.txt"])
            .output()
            .unwrap();
        assert_eq!(get.status.code(), Some(0));
        assert_eq!(get.stdout, "  中文\\folder\n\tsecond\ntail \n\n".as_bytes());
        assert_eq!(fs::read(dir.path().join("descript.ion")).unwrap(),
            "\u{feff}\r\nother keep\n\"照片 😀.txt\"   中文\\\\folder\\n\tsecond\\ntail \\n\\n\u{4}\u{c2}\r\n".as_bytes());
        assert!(!edit_path(dir.path()).exists());
    }
}

#[test]
fn normalized_edit_noop_keeps_original_bytes_and_time() {
    let original = "\u{feff}\"照片 😀.txt\" a\\nb\\n\u{4}\u{c2}\n";
    let dir = fixture(Some(original.as_bytes()));
    let file = dir.path().join("descript.ion");
    let time = fs::metadata(&file).unwrap().modified().unwrap();
    fs::write(dir.path().join("body.txt"), "\u{feff}a\r\nb\r").unwrap();
    changed(
        &edit(dir.path())
            .env("EDIT_BODY", "body.txt")
            .output()
            .unwrap(),
        false,
    );
    assert_eq!(fs::read(&file).unwrap(), original.as_bytes());
    assert_eq!(fs::metadata(file).unwrap().modified().unwrap(), time);
}

#[test]
fn invalid_edit_input_is_retained_and_never_committed() {
    let original = "\u{feff}\"照片 😀.txt\" old\r\n";
    for (body, exit, code) in [
        (b" \t\r\n".as_slice(), 2, "invalid_argument"),
        (b"\xff", 1, "invalid_encoding"),
        (b"\xef\xbb\xbf\xff", 1, "invalid_encoding"),
        (b"bad\0body", 2, "invalid_argument"),
        (b"bad\x04body", 2, "invalid_argument"),
    ] {
        let dir = fixture(Some(original.as_bytes()));
        fs::write(dir.path().join("body.txt"), body).unwrap();
        assert_eq!(
            rejected(
                &edit(dir.path())
                    .env("EDIT_BODY", "body.txt")
                    .output()
                    .unwrap(),
                exit,
                code,
                dir.path()
            ),
            body
        );
        assert_eq!(
            fs::read(dir.path().join("descript.ion")).unwrap(),
            original.as_bytes()
        );
    }
}

#[test]
fn nonempty_unchanged_body_still_validates_input_target_and_extension() {
    for (original, target_exists, exit, code) in [
        (
            "\u{feff}\"照片 😀.txt\"  \t\r\n",
            true,
            2,
            "invalid_argument",
        ),
        ("\u{feff}\"照片 😀.txt\" old\r\n", false, 1, "io_error"),
        (
            "\u{feff}\"照片 😀.txt\" old\u{4}opaque\r\n",
            true,
            1,
            "unknown_extension",
        ),
    ] {
        let dir = fixture(Some(original.as_bytes()));
        if !target_exists {
            fs::remove_file(dir.path().join("照片 😀.txt")).unwrap();
        }
        rejected(&edit(dir.path()).output().unwrap(), exit, code, dir.path());
        assert_eq!(
            fs::read(dir.path().join("descript.ion")).unwrap(),
            original.as_bytes()
        );
    }
}

#[test]
fn launch_and_nonzero_exit_preserve_edit_text_without_commit() {
    let original = "\u{feff}\"照片 😀.txt\" old\r\n";
    let dir = fixture(Some(original.as_bytes()));
    assert_eq!(
        rejected(
            &edit(dir.path())
                .env("VISUAL", "missing-dion-editor.exe")
                .output()
                .unwrap(),
            1,
            "io_error",
            dir.path()
        ),
        b"old"
    );
    fs::write(dir.path().join("body.txt"), b"edited but failed").unwrap();
    assert_eq!(
        rejected(
            &edit(dir.path())
                .env("EDIT_BODY", "body.txt")
                .env("EDIT_EXIT", "7")
                .output()
                .unwrap(),
            1,
            "io_error",
            dir.path()
        ),
        b"edited but failed"
    );
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn visual_precedes_editor_and_empty_visual_falls_back_to_editor() {
    for visual in [None, Some(""), Some("configured")] {
        let dir = fixture(None);
        let configuration = format!("\"{}\" --label \"quoted value\"", editor().display());
        let mut cmd = edit(dir.path());
        match visual {
            Some("configured") => {
                cmd.env("EDITOR", "missing-editor.exe");
            }
            Some(value) => {
                cmd.env("VISUAL", value).env("EDITOR", &configuration);
            }
            None => {
                cmd.env_remove("VISUAL").env("EDITOR", &configuration);
            }
        }
        changed(&cmd.output().unwrap(), false);
        assert!(!edit_path(dir.path()).exists());
    }
}

#[test]
fn preserves_editor_argument_quoting_and_unicode_program_path() {
    let dir = fixture(None);
    let tools = dir.path().join("编辑器 tools");
    fs::create_dir(&tools).unwrap();
    let exe = tools.join("my editor.exe");
    fs::copy(editor(), &exe).unwrap();
    let configuration = format!(r#""{}" --label "quoted \"value\"""#, exe.display());
    changed(
        &edit(dir.path())
            .env("VISUAL", configuration)
            .env("EDIT_EXPECT_LABEL", "quoted \"value\"")
            .output()
            .unwrap(),
        false,
    );
}

#[test]
fn waits_for_editor_exit_before_reading_and_committing() {
    use std::{
        process::Stdio,
        thread,
        time::{Duration, Instant},
    };
    let original = "\u{feff}\"照片 😀.txt\" old\r\n";
    let dir = fixture(Some(original.as_bytes()));
    fs::write(dir.path().join("body.txt"), b"after wait").unwrap();
    let mut child = edit(dir.path())
        .env("EDIT_GATE", "1")
        .env("EDIT_BODY", "body.txt")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !dir.path().join("started").exists() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!(
                "editor did not start: {:?}",
                child.wait_with_output().unwrap()
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(child.try_wait().unwrap().is_none());
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
    fs::write(dir.path().join("continue"), b"").unwrap();
    changed(&child.wait_with_output().unwrap(), true);
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        "\u{feff}\"照片 😀.txt\" after wait\r\n".as_bytes()
    );
    assert!(!edit_path(dir.path()).exists());
}

#[test]
fn edit_is_a_mutually_exclusive_source_and_only_supported_by_set() {
    let dir = fixture(None);
    for json in [false, true] {
        for args in [
            vec!["set", "照片 😀.txt", "body", "--edit"],
            vec!["set", "照片 😀.txt", "--stdin", "--edit"],
            vec!["set", "照片 😀.txt", "--edit", "--stdin"],
            vec!["set", "照片 😀.txt", "--comment-file", "body.txt", "--edit"],
            vec!["set", "照片 😀.txt", "--edit", "--comment-file", "body.txt"],
            vec!["set", "照片 😀.txt", "--edit", "--edit"],
            vec!["get", "照片 😀.txt", "--edit"],
            vec!["remove", "照片 😀.txt", "--edit"],
            vec!["list", "--edit"],
        ] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_dion"));
            cmd.current_dir(dir.path()).args(args);
            if json {
                cmd.arg("--json");
            }
            let output = cmd.output().unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("invalid_argument"));
            assert!(!dir.path().join("descript.ion").exists());
        }
    }
}

#[test]
fn readonly_description_allows_noop_but_retains_text_on_save_or_delete_failure() {
    let original = "\u{feff}\"照片 😀.txt\" old\r\n";
    for body in [None, Some(b"new".as_slice()), Some(b"".as_slice())] {
        let dir = fixture(Some(original.as_bytes()));
        let file = dir.path().join("descript.ion");
        let original_permissions = fs::metadata(&file).unwrap().permissions();
        let mut permissions = original_permissions.clone();
        permissions.set_readonly(true);
        fs::set_permissions(&file, permissions.clone()).unwrap();
        let mut cmd = edit(dir.path());
        if let Some(body) = body {
            fs::write(dir.path().join("body.txt"), body).unwrap();
            assert_eq!(
                rejected(
                    &cmd.env("EDIT_BODY", "body.txt").output().unwrap(),
                    1,
                    "io_error",
                    dir.path()
                ),
                body
            );
        } else {
            changed(&cmd.output().unwrap(), false);
        }
        assert_eq!(fs::read(&file).unwrap(), original.as_bytes());
        assert!(fs::metadata(&file).unwrap().permissions().readonly());
        fs::set_permissions(file, original_permissions).unwrap();
    }
}

#[test]
fn parses_description_before_creating_text_or_launching_editor() {
    for bytes in [b"no BOM".as_slice(), b"\xef\xbb\xbf\"unclosed"] {
        let dir = fixture(Some(bytes));
        let output = edit(dir.path()).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!dir.path().join("started").exists());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Edit text retained"));
        assert_eq!(fs::read(dir.path().join("descript.ion")).unwrap(), bytes);
    }
}

#[test]
fn text_mode_success_is_silent_and_failure_reports_recovery_path() {
    let dir = fixture(None);
    let mut cmd = edit(dir.path());
    // Replace arguments to exercise text output using the same controlled editor.
    let configuration = cmd
        .get_envs()
        .find(|(key, _)| *key == "VISUAL")
        .unwrap()
        .1
        .unwrap()
        .to_owned();
    cmd = Command::new(env!("CARGO_BIN_EXE_dion"));
    cmd.current_dir(dir.path())
        .args(["set", "照片 😀.txt", "--edit"])
        .env("VISUAL", configuration);
    let output = cmd.output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let output = cmd.env("EDIT_EXIT", "7").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let message = String::from_utf8(output.stderr).unwrap();
    assert!(message.starts_with("io_error: "));
    let path = edit_path(dir.path());
    assert!(message.contains(path.to_str().unwrap()));
    assert!(path.exists());
    fs::remove_file(path).unwrap();
}

#[test]
fn detects_description_changes_even_when_edit_body_is_unchanged() {
    let original = "\u{feff}\"照片 😀.txt\" old\r\n";
    let other = b"\xef\xbb\xbfother concurrent\r\n";
    for action in ["write", "create", "delete"] {
        for body in [None, Some(b"new".as_slice()), Some(b"".as_slice())] {
            let dir = fixture(if action == "create" {
                None
            } else {
                Some(original.as_bytes())
            });
            fs::write(dir.path().join("other.ion"), other).unwrap();
            let mut cmd = edit(dir.path());
            cmd.env(
                "EDIT_CONFLICT",
                if action == "delete" {
                    "delete"
                } else {
                    "write"
                },
            );
            if let Some(body) = body {
                fs::write(dir.path().join("body.txt"), body).unwrap();
                cmd.env("EDIT_BODY", "body.txt");
            }
            rejected(&cmd.output().unwrap(), 1, "content_changed", dir.path());
            let file = dir.path().join("descript.ion");
            if action == "delete" {
                assert!(!file.exists());
            } else {
                assert_eq!(fs::read(file).unwrap(), other);
            }
        }
    }
}
