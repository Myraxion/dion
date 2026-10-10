use std::fs;
use std::process::{Command, Output};

fn run(args: &[&str], language: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
    command.args(args).env_remove("DION_LANG");
    if let Some(language) = language {
        command.env("DION_LANG", language);
    }
    command.output().unwrap()
}

#[test]
fn language_option_selects_help_and_overrides_environment() {
    let output = run(&["--lang", "en", "--help"], Some("zh-CN"));
    assert_eq!(output.status.code(), Some(0));
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("Usage:"));
    assert!(!help.contains("用途："));
}

#[test]
fn language_option_after_an_unknown_option_localizes_the_argument_error() {
    let output = run(&["--unknown", "--lang", "zh-CN"], None);
    assert_eq!(output.status.code(), Some(2));
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("未知") || error.contains("选项"), "{error}");
}

#[test]
fn localized_json_error_keeps_the_machine_contract() {
    let english = run(&["get", "missing.txt", "--lang", "en", "--json"], None);
    let chinese = run(&["get", "missing.txt", "--json", "--lang", "zh-CN"], None);
    assert_eq!(english.status.code(), Some(3));
    assert_eq!(chinese.status.code(), Some(3));
    let english: serde_json::Value = serde_json::from_slice(&english.stderr).unwrap();
    let chinese: serde_json::Value = serde_json::from_slice(&chinese.stderr).unwrap();
    assert_eq!(english["error"]["code"], chinese["error"]["code"]);
    assert_eq!(english["error"]["code"], "not_found");
    assert_eq!(english["error"]["message"], "Comment not found");
    assert_eq!(chinese["error"]["message"], "未找到备注");
    assert_eq!(english["error"]["file"], chinese["error"]["file"]);
}

#[test]
fn list_error_summary_and_json_follow_the_selected_language() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("descript.ion"), b"\xef\xbb\xbf bad\n").unwrap();
    let path = directory.path().to_str().unwrap();
    let english = run(&["list", path, "--lang", "en"], None);
    let chinese = run(&["list", path, "--lang", "zh-CN"], None);
    assert_eq!(english.status.code(), Some(1));
    assert_eq!(chinese.status.code(), Some(1));
    assert!(
        String::from_utf8(english.stdout)
            .unwrap()
            .contains("Errors (1):")
    );
    assert!(
        String::from_utf8(chinese.stdout)
            .unwrap()
            .contains("错误（1）：")
    );

    let english = run(&["list", path, "--lang", "en", "--json"], None);
    let chinese = run(&["list", path, "--lang", "zh-CN", "--json"], None);
    let english: serde_json::Value = serde_json::from_slice(&english.stdout).unwrap();
    let chinese: serde_json::Value = serde_json::from_slice(&chinese.stdout).unwrap();
    assert_eq!(english["errors"][0]["code"], "invalid_format");
    assert_eq!(english["errors"][0]["message"], "Record name is empty");
    assert_eq!(chinese["errors"][0]["message"], "记录名称为空");
}

#[test]
fn first_language_option_wins_and_option_values_are_not_reinterpreted() {
    let repeated = run(&["--lang", "en", "--lang", "zh-CN"], None);
    assert_eq!(repeated.status.code(), Some(2));
    assert!(
        String::from_utf8(repeated.stderr)
            .unwrap()
            .starts_with("invalid_argument:")
    );

    let consumed = run(&["list", "--color", "--lang", "zh-CN"], None);
    assert_eq!(consumed.status.code(), Some(2));
    assert!(
        String::from_utf8(consumed.stderr)
            .unwrap()
            .starts_with("invalid_argument:")
    );

    let consumed = run(
        &["set", "target", "--comment-file", "--lang", "zh-CN"],
        Some("en"),
    );
    assert_eq!(consumed.status.code(), Some(2));
    assert!(
        String::from_utf8(consumed.stderr)
            .unwrap()
            .contains("prefix a filename")
    );
}

#[test]
fn auto_overrides_the_environment_and_terminator_keeps_values_literal() {
    let explicit_auto = run(&["--lang", "auto", "--help"], Some("en"));
    let system_auto = run(&["--lang", "auto", "--help"], None);
    assert_eq!(explicit_auto.stdout, system_auto.stdout);

    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("target.txt"), b"entry").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
    let result = command
        .current_dir(directory.path())
        .args(["set", "--lang", "en", "target.txt", "--", "--lang"])
        .env_remove("DION_LANG")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        fs::read(directory.path().join("descript.ion")).unwrap(),
        b"\xef\xbb\xbf\r\ntarget.txt --lang\r\n"
    );
}
