use std::{path::Path, process::Command};

fn help(directory: &Path, args: &[&str]) -> String {
    let result = Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .env("DION_LANG", "zh-CN")
        .env("VISUAL", "dion-help-editor-must-not-run.exe")
        .env("EDITOR", "dion-help-editor-must-not-run.exe")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0), "{args:?}: {result:?}");
    assert!(result.stderr.is_empty(), "{args:?}");
    assert!(!result.stdout.starts_with(b"\xef\xbb\xbf"));
    assert!(result.stdout.ends_with(b"\n"));
    assert!(serde_json::from_slice::<serde_json::Value>(&result.stdout).is_err());
    String::from_utf8(result.stdout).unwrap()
}

#[test]
fn help_does_not_read_or_modify_comments_or_start_an_editor() {
    use std::fs;
    for bytes in [
        b"\xef\xbb\xbftarget original\r\n".as_slice(),
        b"invalid UTF-8: \xff",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let description = directory.path().join("descript.ion");
        fs::write(&description, bytes).unwrap();
        let modified = fs::metadata(&description).unwrap().modified().unwrap();
        for args in [
            vec!["--help"],
            vec!["get", "absent-parent/target", "-h"],
            vec!["list", "absent-directory", "--recursive", "--help"],
            vec!["list", "--tree", "-h"],
            vec!["set", "target", "replacement", "--help"],
            vec!["set", "target", "--edit", "--help", "--json"],
            vec!["set", "target", "--stdin", "-h"],
            vec!["set", "target", "--comment-file", "absent.txt", "-h"],
            vec!["remove", "target", "--help"],
            vec!["help", "set"],
        ] {
            help(directory.path(), &args);
            assert_eq!(fs::read(&description).unwrap(), bytes);
            assert_eq!(
                fs::metadata(&description).unwrap().modified().unwrap(),
                modified
            );
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn help_names_after_separator_remain_paths_and_comment_text() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("--help"), b"target").unwrap();
    for args in [
        vec!["set", "--", "--help", "-h"],
        vec!["get", "--", "--help"],
        vec!["remove", "--", "--help"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_dion"))
            .current_dir(directory.path())
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty());
        assert_eq!(
            output.stdout,
            if args[0] == "get" {
                b"-h".as_slice()
            } else {
                b""
            }
        );
    }
    assert!(!directory.path().join("descript.ion").exists());
}

#[test]
fn invalid_help_topics_and_ordinary_argument_errors_keep_the_error_contract() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec!["help", "unknown"],
        vec!["help", "get", "extra"],
        vec!["unknown", "--help"],
        vec!["get"],
        vec!["set", "target"],
        vec!["list", "--tree", "--long"],
        vec!["set", "target", "--stdin", "--edit"],
    ] {
        for json in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
            command.current_dir(directory.path()).args(&args);
            if json {
                command.arg("--json");
            }
            let output = command.output().unwrap();
            assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
            assert!(output.stdout.is_empty());
            if json {
                let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"]["code"], "invalid_argument");
            } else {
                assert!(output.stderr.starts_with(b"invalid_argument:"));
            }
        }
    }
}

#[test]
fn overview_routes_show_chinese_usage_options_and_key_defaults() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec!["--help"],
        vec!["-h"],
        vec!["help"],
        vec!["--json", "--help"],
        vec!["-h", "--json"],
        vec!["help", "--json"],
    ] {
        let text = help(directory.path(), &args);
        assert!(!text.contains("--lang"));
        for expected in [
            "用途",
            "用法",
            "参数",
            "默认",
            "get",
            "list",
            "set",
            "remove",
            "--json",
            "--help",
            "-h",
            "当前目录",
            "--",
            "条目自身",
            "目录内部",
        ] {
            assert!(text.contains(expected), "{args:?}: missing {expected}");
        }
        assert!(!text.contains("示例"));
    }
    assert!(
        std::fs::read_dir(directory.path())
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn english_overview_does_not_expose_language_selection() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory.path())
        .args(["--help"])
        .env("DION_LANG", "en")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!String::from_utf8(output.stdout).unwrap().contains("--lang"));
}

#[test]
fn every_command_has_three_help_routes_without_required_operands() {
    let directory = tempfile::tempdir().unwrap();
    for (command, details) in [
        (
            "get",
            vec!["<path>", "父目录", "只输出正文", "未找到", "3", "--json"],
        ),
        (
            "list",
            vec![
                "[directory]",
                "当前目录",
                "双栏",
                "--long",
                "-l",
                "--recursive",
                "-r",
                "--all",
                "-a",
                "$RECYCLE.BIN",
                ".git",
                "--tree",
                "--color",
                "always",
                "自动递归",
                "互斥",
                "目录优先",
                "自然排序",
                "符号链接",
                "Junction",
                "起始目录",
                "普通递归 JSON",
            ],
        ),
        (
            "set",
            vec![
                "<path>",
                "<comment>",
                "--stdin",
                "--comment-file",
                "--edit",
                "四种",
                "互斥",
                "VISUAL",
                "EDITOR",
                "Windows 记事本",
                "正常退出",
                "清空",
                "纯空白",
                "恢复路径",
                "UTF-8",
                "BOM",
                "changed",
            ],
        ),
        (
            "remove",
            vec![
                "<path>",
                "无需目标存在",
                "无记录",
                "最后一条",
                "静默",
                "changed",
            ],
        ),
    ] {
        let baseline = help(directory.path(), &["help", command]);
        for args in [
            vec![command, "--help"],
            vec![command, "-h"],
            vec!["help", command],
            vec!["--json", command, "--help"],
            vec![command, "-h", "--json"],
            vec!["help", command, "--json"],
        ] {
            let text = help(directory.path(), &args);
            assert_eq!(text, baseline);
            for expected in ["用途", "用法", "参数", "默认", "--json", "--help", "-h"]
                .into_iter()
                .chain(details.iter().copied())
            {
                assert!(text.contains(expected), "{args:?}: missing {expected}");
            }
            assert!(!text.contains("示例"));
        }
    }
}

#[test]
fn aliases_use_canonical_help_and_show_all_spellings() {
    let directory = tempfile::tempdir().unwrap();
    let overview = help(directory.path(), &["-j", "-h"]);
    for (command, aliases) in [
        ("get", vec!["view", "cat"]),
        ("list", vec!["ls"]),
        ("remove", vec!["rm", "unset", "del"]),
    ] {
        let baseline = help(directory.path(), &["help", command]);
        for alias in aliases {
            assert!(overview.contains(alias));
            assert!(baseline.contains(alias));
            for args in [
                vec!["help", alias],
                vec![alias, "-h"],
                vec![alias, "--help"],
                vec!["-j", alias, "-h"],
                vec!["help", alias, "-j"],
            ] {
                assert_eq!(help(directory.path(), &args), baseline);
            }
        }
    }
    for (command, options) in [
        ("get", vec!["--json, -j"]),
        ("list", vec!["--tree, -t", "--json, -j"]),
        (
            "set",
            vec![
                "--stdin, -i",
                "--comment-file, -f <file>",
                "--edit, -e",
                "--json, -j",
            ],
        ),
        ("remove", vec!["--json, -j"]),
    ] {
        let text = help(directory.path(), &[command, "-h"]);
        for option in options {
            assert!(text.contains(option), "{command}: missing {option}");
        }
    }
}
