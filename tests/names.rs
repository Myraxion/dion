#![cfg(windows)]

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn fixture(contents: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("descript.ion"), contents).unwrap();
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn reads_distinct_ordinal_names_without_lowercase_duplicate_conflicts() {
    // Known results from the Windows ordinal comparison probe, not lowercase keys.
    let original = "\u{feff}k.txt latin\r\nK.txt kelvin\r\nẞ.txt capital\r\nß.txt small\r\nİ.txt dotted\r\ni\u{307}.txt combined\r\n\u{10400}.txt deseret-capital\r\n\u{10428}.txt deseret-small\r\n";
    let dir = fixture(original);
    for (name, body) in [
        ("K.TXT", "latin"),
        ("K.txt", "kelvin"),
        ("ẞ.txt", "capital"),
        ("ß.txt", "small"),
        ("İ.txt", "dotted"),
        ("i\u{307}.txt", "combined"),
        ("\u{10400}.txt", "deseret-capital"),
        ("\u{10428}.txt", "deseret-small"),
    ] {
        let output = run(dir.path(), &["get", name]);
        assert_eq!(output.status.code(), Some(0), "{name}: {:?}", output.stderr);
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout, body.as_bytes(), "{name}");
    }
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn sets_ordinal_target_preserving_spelling_and_other_record_bytes() {
    let dir = fixture("\u{feff}K.txt keep kelvin\nk.txt old\rother untouched");
    fs::write(dir.path().join("k.txt"), b"entry").unwrap();
    let output = run(dir.path(), &["set", "K.TXT", "updated", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"changed": true})
    );
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        "\u{feff}K.txt keep kelvin\nk.txt updated\r\nother untouched".as_bytes()
    );
}

#[test]
fn removes_only_the_ordinal_target_and_keeps_other_bytes() {
    let dir = fixture("\u{feff}K.txt keep kelvin\nk.txt remove\rother untouched");
    let output = run(dir.path(), &["remove", "K.TXT", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"changed": true})
    );
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        "\u{feff}K.txt keep kelvin\nother untouched".as_bytes()
    );
}

#[test]
fn identifies_directories_without_conflating_ordinal_names() {
    let original = "\u{feff}K orphan\nk directory\r\n";
    let dir = fixture(original);
    fs::create_dir(dir.path().join("k")).unwrap();
    let output = run(dir.path(), &["list", "--long"]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(
        output.stdout,
        "k\\\n    directory\n\nK\n    orphan\n\n".as_bytes()
    );
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn tree_keeps_children_with_their_distinct_ordinal_directory_records() {
    let dir = fixture("\u{feff}K kelvin\nk latin\n");
    for (name, record) in [
        ("k", "latin.txt from latin"),
        ("K", "kelvin.txt from kelvin"),
    ] {
        fs::create_dir(dir.path().join(name)).unwrap();
        fs::write(
            dir.path().join(name).join("descript.ion"),
            format!("\u{feff}{record}\n"),
        )
        .unwrap();
    }
    let output = run(dir.path(), &["list", "--tree"]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, "├── k\\  latin\n│   └── latin.txt  from latin\n└── K\\  kelvin\n    └── kelvin.txt  from kelvin\n".as_bytes());
}

#[test]
fn missing_ordinal_match_is_not_read_or_removed_through_a_lowercase_alias() {
    let original = "\u{feff}K.txt keep\r\n";
    let dir = fixture(original);
    let output = run(dir.path(), &["get", "K.TXT", "--json"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["error"]["code"],
        "not_found"
    );
    let output = run(dir.path(), &["remove", "K.TXT", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"changed": false})
    );
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn ordinal_duplicate_conflicts_preserve_strict_commands_and_list_diagnostics() {
    let original = "\u{feff}K.txt kelvin\r\nk.txt first\r\nK.TXT duplicate\r\n";
    let dir = fixture(original);
    fs::write(dir.path().join("k.txt"), b"entry").unwrap();
    for args in [
        vec!["get", "k.txt", "--json"],
        vec!["set", "k.txt", "updated", "--json"],
        vec!["remove", "k.txt", "--json"],
    ] {
        let output = run(dir.path(), &args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_format");
        assert_eq!(error["error"]["line"], 3);
        assert_eq!(
            fs::read(dir.path().join("descript.ion")).unwrap(),
            original.as_bytes()
        );
    }
    let output = run(dir.path(), &["list", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let listing: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        listing["entries"],
        serde_json::json!([
            {"name":"k.txt", "comment":"first", "extension":"none"},
            {"name":"K.txt", "comment":"kelvin", "extension":"none"},
        ])
    );
    assert_eq!(listing["errors"].as_array().unwrap().len(), 1);
    assert_eq!(listing["errors"][0]["code"], "invalid_format");
    assert_eq!(listing["errors"][0]["line"], 3);
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn compares_complete_record_names_without_nul_truncation_or_normalization() {
    let original = "\u{feff}a\0x one\na\0y two\nÅ.txt composed\nA\u{30a}.txt decomposed\n";
    let dir = fixture(original);
    let output = run(dir.path(), &["list", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let listing: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(listing["errors"], serde_json::json!([]));
    let entries = listing["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    for (name, body) in [
        ("a\0x", "one"),
        ("a\0y", "two"),
        ("Å.txt", "composed"),
        ("A\u{30a}.txt", "decomposed"),
    ] {
        assert!(
            entries
                .iter()
                .any(|entry| entry["name"] == name && entry["comment"] == body)
        );
    }
    let output = run(dir.path(), &["get", "a"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        fs::read(dir.path().join("descript.ion")).unwrap(),
        original.as_bytes()
    );
}
