use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

fn assert_same(actual: &Output, expected: &Output) {
    assert_eq!(actual.status.code(), expected.status.code());
    assert_eq!(actual.stdout, expected.stdout);
    assert_eq!(actual.stderr, expected.stderr);
}

#[test]
fn viewing_aliases_match_text_json_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let original = "\u{feff}cat 中文\nview 原样路径\n";
    fs::write(dir.path().join("descript.ion"), original).unwrap();
    for alias in ["view", "cat"] {
        for operands in [vec!["cat"], vec!["view", "--json"], vec!["missing"], vec![]] {
            let mut canonical = vec!["get"];
            canonical.extend_from_slice(&operands);
            let mut short = vec![alias];
            short.extend_from_slice(&operands);
            assert_same(&run(dir.path(), &short), &run(dir.path(), &canonical));
        }
    }
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn list_alias_accepts_list_options_and_short_tree_matches_recursive_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("child")).unwrap();
    fs::write(dir.path().join("descript.ion"), "\u{feff}root 根备注\n").unwrap();
    fs::write(
        dir.path().join("child/descript.ion"),
        "\u{feff}leaf 子备注\n",
    )
    .unwrap();
    for (short, full) in [
        (vec!["ls"], vec!["list"]),
        (
            vec!["ls", "-l", "-r"],
            vec!["list", "--long", "--recursive"],
        ),
        (vec!["ls", "-t"], vec!["list", "--tree"]),
        (
            vec!["-j", "ls", "-t", "-r"],
            vec!["list", "--recursive", "--json"],
        ),
    ] {
        let expected = run(dir.path(), &full);
        assert_eq!(expected.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&expected.stdout).contains("子备注") || full == ["list"]);
        assert_same(&run(dir.path(), &short), &expected);
    }
}

#[test]
fn removal_aliases_delete_only_comments_and_keep_noop_results() {
    for alias in ["rm", "unset", "del"] {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("cat"), b"target bytes").unwrap();
        fs::write(dir.path().join("descript.ion"), "\u{feff}cat note\n").unwrap();
        for changed in [true, false] {
            let output = run(dir.path(), &[alias, "cat", "-j"]);
            assert_eq!(output.status.code(), Some(0), "{alias}: {output:?}");
            assert!(output.stderr.is_empty());
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
                serde_json::json!({"changed": changed})
            );
            assert!(!dir.path().join("descript.ion").exists());
            assert_eq!(fs::read(dir.path().join("cat")).unwrap(), b"target bytes");
        }
    }
}

#[test]
fn short_json_formats_success_operation_errors_and_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("descript.ion"), "\u{feff}target note\n").unwrap();
    for args in [
        vec!["cat", "target"],
        vec!["view", "missing"],
        vec!["cat"],
        vec!["unknown"],
        vec!["ls", "--unknown"],
        vec!["ls", "-t", "-l"],
        vec!["cat", "target", "--json"],
    ] {
        let mut full = args.clone();
        full.insert(0, "--json");
        for short in [[&["-j"][..], &args].concat(), [&args[..], &["-j"]].concat()] {
            assert_same(&run(dir.path(), &short), &run(dir.path(), &full));
        }
    }
}

#[test]
fn short_options_keep_repetition_exclusivity_scope_and_exact_syntax() {
    let dir = tempfile::tempdir().unwrap();
    let original = "\u{feff}target old\n";
    fs::write(dir.path().join("descript.ion"), original).unwrap();
    for args in [
        vec!["ls", "--tree", "-t"],
        vec!["ls", "-t", "-t"],
        vec!["ls", "-l", "-t"],
        vec!["cat", "target", "-t"],
        vec!["rm", "target", "-r"],
        vec!["ls", "-i"],
        vec!["cat", "target", "-e"],
        vec!["del", "target", "-f", "body.txt"],
        vec!["set", "target", "--stdin", "-i"],
        vec!["set", "target", "--edit", "-e"],
        vec![
            "set",
            "target",
            "--comment-file",
            "body.txt",
            "-f",
            "body.txt",
        ],
        vec!["set", "target", "-i", "-e"],
        vec!["set", "target", "-f", "body.txt", "-i"],
        vec!["set", "target", "body", "-e"],
        vec!["set", "target", "-f"],
        vec!["set", "target", "-f", "-i"],
        vec!["ls", "-lr"],
        vec!["set", "target", "-fbody.txt"],
        vec!["set", "target", "-f=body.txt"],
        vec!["set", "target", "--comment-file=body.txt"],
        vec!["ca", "target"],
        vec!["CAT", "target"],
        vec!["s", "target", "body"],
        vec!["g", "target"],
    ] {
        let output = run(dir.path(), &[&["-j"][..], &args].concat());
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["error"]["code"],
            "invalid_argument"
        );
        assert_eq!(
            fs::read(dir.path().join("descript.ion")).unwrap(),
            original.as_bytes()
        );
    }
}

#[test]
fn aliases_and_short_options_stay_literal_in_operands_and_after_separator() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("-j"), b"target").unwrap();
    fs::write(dir.path().join("ls"), b"cat").unwrap();
    let output = run(dir.path(), &["set", "-f", "ls", "--", "-j"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty());
    let output = run(dir.path(), &["view", "--", "-j"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"cat");
    let output = run(dir.path(), &["set", "--", "-j", "-e"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let output = run(dir.path(), &["cat", "--", "-j"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"-e");
}
