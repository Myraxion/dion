use std::{path::Path, process::Command};

fn run(directory: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .env("DION_LANG", "en")
        .env("VISUAL", "dion-version-editor-must-not-run.exe")
        .env("EDITOR", "dion-version-editor-must-not-run.exe")
        .output()
        .unwrap()
}

#[test]
fn version_routes_report_the_package_version_as_plain_text() {
    let directory = tempfile::tempdir().unwrap();
    // Cargo.toml is the source of truth for the public package version.
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .unwrap()
        .trim_end_matches('"');
    for args in [
        vec!["--version"],
        vec!["-V"],
        vec!["--json", "--lang", "en", "-V"],
        vec!["--version", "--lang", "zh-CN", "-j"],
        vec!["--lang", "auto", "--version"],
    ] {
        let output = run(directory.path(), &args);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
        assert_eq!(output.stdout, format!("dion {version}\n").as_bytes());
    }
}

#[test]
fn version_rejects_commands_help_and_operation_options_without_side_effects() {
    let directory = tempfile::tempdir().unwrap();
    let description = directory.path().join("descript.ion");
    let original = b"\xef\xbb\xbftarget original\r\n";
    std::fs::write(&description, original).unwrap();
    let modified = std::fs::metadata(&description).unwrap().modified().unwrap();
    for args in [
        vec!["--version", "--help"],
        vec!["-h", "-V"],
        vec!["help", "--version"],
        vec!["get", "absent/target", "--version"],
        vec!["list", "absent", "-V"],
        vec!["set", "target", "replacement", "--version"],
        vec!["remove", "target", "-V"],
        vec!["set", "target", "--edit", "-V"],
        vec!["--version", "--stdin"],
        vec!["--version", "--comment-file", "absent.txt"],
        vec!["--long", "--version"],
        vec!["-r", "-V"],
        vec!["--tree", "-V"],
        vec!["--all", "-V"],
        vec!["--color", "never", "-V"],
        vec!["--everything", "-V"],
        vec!["--everything-instance", "example", "-V"],
        vec!["--version", "extra"],
        vec!["--version", "-V"],
        vec!["-V", "-V"],
        vec!["--version", "--unknown"],
        vec!["--version", "--lang", "unknown"],
        vec!["--version", "--lang"],
        vec!["--version", "--lang", "en", "--lang", "zh-CN"],
    ] {
        for json in [false, true] {
            let mut args = args.clone();
            if json {
                args.insert(0, "--json");
            }
            let output = run(directory.path(), &args);
            assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
            assert!(output.stdout.is_empty(), "{args:?}");
            if json {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"]["code"], "invalid_argument");
            } else {
                assert!(output.stderr.starts_with(b"invalid_argument:"));
            }
            assert_eq!(std::fs::read(&description).unwrap(), original);
            assert_eq!(
                std::fs::metadata(&description).unwrap().modified().unwrap(),
                modified
            );
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn version_names_after_separator_remain_paths_and_comment_text() {
    let directory = tempfile::tempdir().unwrap();
    for (name, comment) in [("--version", "-V"), ("-V", "--version")] {
        std::fs::write(directory.path().join(name), b"target").unwrap();
        for args in [
            vec!["set", "--", name, comment],
            vec!["get", "--", name],
            vec!["remove", "--", name],
        ] {
            let output = run(directory.path(), &args);
            assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
            assert!(output.stderr.is_empty());
            assert_eq!(
                output.stdout,
                if args[0] == "get" {
                    comment.as_bytes()
                } else {
                    b""
                }
            );
        }
    }
    assert!(!directory.path().join("descript.ion").exists());
}

#[test]
fn version_succeeds_without_reading_or_modifying_an_invalid_description_file() {
    let directory = tempfile::tempdir().unwrap();
    let description = directory.path().join("descript.ion");
    let original = b"invalid UTF-8: \xff";
    std::fs::write(&description, original).unwrap();
    let modified = std::fs::metadata(&description).unwrap().modified().unwrap();
    let output = run(directory.path(), &["--version"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read(&description).unwrap(), original);
    assert_eq!(
        std::fs::metadata(&description).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
