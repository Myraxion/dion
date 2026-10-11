use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn list(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .env("DION_LANG", "zh-CN")
        .env_remove("NO_COLOR")
        .output()
        .unwrap()
}

fn list_with_no_color(directory: &Path, args: &[&str], value: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .env("DION_LANG", "zh-CN")
        .env("NO_COLOR", value)
        .output()
        .unwrap()
}

#[test]
fn color_always_styles_names_in_every_text_layout_but_not_json() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("folder")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}file note\r\nfolder directory note",
    )
    .unwrap();
    fs::write(
        directory.path().join("folder/descript.ion"),
        "\u{feff}nested child note",
    )
    .unwrap();

    for (options, expected) in [
        (
            vec!["--color", "always"],
            "\u{1b}[36mfolder\\\u{1b}[39m  directory note\n\u{1b}[36mfile\u{1b}[39m     note\n",
        ),
        (
            vec!["--color", "always", "--long"],
            "\u{1b}[36mfolder\\\u{1b}[39m\n    directory note\n\n\u{1b}[36mfile\u{1b}[39m\n    note\n\n",
        ),
        (
            vec!["--color", "always", "--tree"],
            "\u{1b}[36m├──\u{1b}[39m \u{1b}[36mfolder\\\u{1b}[39m  directory note\n\u{1b}[36m│\u{1b}[39m   \u{1b}[36m└──\u{1b}[39m \u{1b}[36mnested\u{1b}[39m  child note\n\u{1b}[36m└──\u{1b}[39m \u{1b}[36mfile\u{1b}[39m  note\n",
        ),
    ] {
        let mut args = vec!["list"];
        args.extend(options);
        let result = list(directory.path(), &args);
        assert_eq!(
            result.status.code(),
            Some(0),
            "{args:?}: {:?}",
            result.stderr
        );
        assert_eq!(result.stdout, expected.as_bytes(), "{args:?}");
    }

    let result = list(directory.path(), &["list", "--color", "always", "--json"]);
    assert_eq!(result.status.code(), Some(0));
    assert!(!result.stdout.windows(2).any(|window| window == b"\x1b["));
    assert!(serde_json::from_slice::<serde_json::Value>(&result.stdout).is_ok());

    let canonical = list(directory.path(), &["list", "--color", "always"]);
    let alias = list(directory.path(), &["ls", "--color", "always"]);
    assert_eq!(alias.stdout, canonical.stdout);
}

#[test]
fn recursive_list_skips_default_directories_and_all_restores_them() {
    let directory = tempfile::tempdir().unwrap();
    let excluded = [
        "$RECYCLE.BIN",
        "System Volume Information",
        ".git",
        "node_modules",
        ".venv",
        "__pycache__",
        ".pytest_cache",
        ".next",
        ".svn",
        ".mypy_cache",
        ".ruff_cache",
        ".tox",
        ".nox",
        ".parcel-cache",
    ];
    let mut root_records = String::from("\u{feff}visible visible directory note\n");
    for name in excluded {
        let child = directory.path().join(name);
        fs::create_dir(&child).unwrap();
        fs::write(
            child.join("descript.ion"),
            format!("\u{feff}inside {name} note"),
        )
        .unwrap();
        root_records.push_str(&format!("{name} {name} directory note\n"));
    }
    let nested = directory.path().join("visible/.VeNv");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("descript.ion"), "\u{feff}deep deep note").unwrap();
    root_records.push_str(".git-backup ordinary directory note\n");
    fs::create_dir(directory.path().join(".git-backup")).unwrap();
    fs::write(directory.path().join("descript.ion"), root_records).unwrap();

    let result = list(directory.path(), &["list", "-r", "--json"]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let names: Vec<_> = value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&".git"));
    assert!(names.contains(&"visible"));
    assert!(names.contains(&".git-backup"));
    assert!(!names.contains(&"visible\\.VeNv\\deep"));
    assert!(!names.iter().any(|name| name.contains("\\inside")));
    assert_eq!(value["errors"], serde_json::json!([]));

    let result = list(directory.path(), &["list", "--tree"]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    let tree = String::from_utf8(result.stdout).unwrap();
    assert!(tree.contains(".git\\  .git directory note"));
    assert!(!tree.contains("inside"));

    let result = list(directory.path(), &["list", "--tree", "--all", "--json"]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        value["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["name"] == "$RECYCLE.BIN\\inside" })
    );

    let result = list(directory.path(), &["list", "-r", "--all", "--json"]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    for name in excluded {
        assert!(
            value["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| { entry["name"] == format!("{name}\\inside") })
        );
    }
    assert!(
        value["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["name"] == "visible\\.VeNv\\deep" })
    );

    let result = list(directory.path(), &["list", "-a", "--json"]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        value["entries"].as_array().unwrap().len(),
        excluded.len() + 2
    );
}

#[test]
fn explicitly_listed_default_exclusion_is_read_but_its_children_remain_excluded() {
    let directory = tempfile::tempdir().unwrap();
    let git = directory.path().join(".git");
    fs::create_dir_all(git.join("node_modules")).unwrap();
    fs::write(git.join("descript.ion"), "\u{feff}config config note").unwrap();
    fs::write(
        git.join("node_modules/descript.ion"),
        "\u{feff}inside nested note",
    )
    .unwrap();

    let result = list(directory.path(), &["list", ".git", "-r", "--json"]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["entries"][0]["name"], "config");
    assert_eq!(value["entries"].as_array().unwrap().len(), 1);
}

#[test]
fn automatic_and_disabled_color_keep_captured_output_plain_and_no_color_is_respected() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("descript.ion"), "\u{feff}name note").unwrap();
    let expected = b"name  note\n";

    for args in [
        vec!["list"],
        vec!["list", "--color", "auto"],
        vec!["list", "--color", "never"],
    ] {
        let result = list(directory.path(), &args);
        assert_eq!(result.status.code(), Some(0), "{args:?}");
        assert_eq!(result.stdout, expected, "{args:?}");
    }

    for value in ["1", "false"] {
        let result = list_with_no_color(directory.path(), &["list"], value);
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, expected);
    }

    let result = list_with_no_color(directory.path(), &["list", "--color", "always"], "1");
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"\x1b[36mname\x1b[39m  note\n");
}

#[test]
fn colored_names_restore_default_foreground_before_unmodified_comment_controls() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}name \u{1b}[31mred",
    )
    .unwrap();

    let result = list(directory.path(), &["list", "--color", "always"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"\x1b[36mname\x1b[39m  \x1b[31mred\n");
}

#[test]
fn tree_colors_comment_continuation_guides_without_coloring_comment_or_indentation() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("folder")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}folder first\\nsecond\u{4}\u{c2}\r\nother note",
    )
    .unwrap();

    let result = list(directory.path(), &["list", "--tree", "--color", "always"]);
    assert_eq!(result.status.code(), Some(0));
    let expected = format!(
        "\u{1b}[36m├──\u{1b}[39m \u{1b}[36mfolder\\\u{1b}[39m  first\n\u{1b}[36m│\u{1b}[39m{}second\n\u{1b}[36m└──\u{1b}[39m \u{1b}[36mother\u{1b}[39m  note\n",
        " ".repeat(12),
    );
    assert_eq!(result.stdout, expected.as_bytes());
}

#[test]
fn color_always_covers_supporting_nodes_and_orphans_without_coloring_errors() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("folder/helper/deep")).unwrap();
    let mut root = b"\xef\xbb\xbffolder directory note\r\norphan lost note\r\n".to_vec();
    root.extend_from_slice("raw\u{1b}[31m raw escape name\r\n".as_bytes());
    root.extend_from_slice(b"bad \xff");
    fs::write(directory.path().join("descript.ion"), root).unwrap();
    fs::write(
        directory.path().join("folder/helper/deep/descript.ion"),
        "\u{feff}absent deep note",
    )
    .unwrap();

    let result = list(directory.path(), &["list", "--tree", "--color", "always"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.is_empty());
    let text = String::from_utf8(result.stdout).unwrap();
    for name in ["folder\\", "helper\\", "deep\\", "absent", "orphan"] {
        assert!(
            text.contains(&format!("\u{1b}[36m{name}\u{1b}[39m")),
            "missing colored name {name:?}: {text:?}"
        );
    }
    assert!(text.contains("\u{1b}[36mraw\u{1b}[31m\u{1b}[39m"));
    let (_, errors) = text.split_once("错误（1）：\n").unwrap();
    assert!(errors.contains("invalid_encoding"));
    assert!(!errors.contains("\u{1b}["));
}

#[test]
fn colored_wide_names_keep_multiline_alignment_and_empty_comments() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}\"照片 😀\" first\\nsecond\u{4}\u{c2}\r\nempty \r\n",
    )
    .unwrap();

    let result = list(directory.path(), &["list", "--color", "always"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        result.stdout,
        concat!(
            "\u{1b}[36mempty\u{1b}[39m    \n",
            "\u{1b}[36m照片 😀\u{1b}[39m  first\n",
            "         second\n",
        )
        .as_bytes()
    );
}

#[test]
fn tree_merges_directory_comments_and_prunes_unrelated_nodes_in_natural_order() {
    let directory = tempfile::tempdir().unwrap();
    for path in [
        "dir2/middle/deep",
        "dir2/unused",
        "dir10",
        "irrelevant/nested",
    ] {
        fs::create_dir_all(directory.path().join(path)).unwrap();
    }
    fs::write(directory.path().join("unrelated.txt"), b"").unwrap();
    let fixtures = [
        (
            "descript.ion",
            "\u{feff}file10 ten\r\nDIR2  first\\n\\n tail \\n\\n\u{4}\u{c2}\r\nfile2 two\r\nempty ",
        ),
        ("dir2/descript.ion", "\u{feff}orphan missing"),
        (
            "dir2/middle/deep/descript.ion",
            "\u{feff}lost literal\\n\u{4}foreign",
        ),
        ("dir10/descript.ion", "\u{feff}file last"),
    ];
    for (path, bytes) in fixtures {
        fs::write(directory.path().join(path), bytes).unwrap();
    }
    for options in [
        vec!["--tree"],
        vec!["--tree", "-r"],
        vec!["--recursive", "--tree"],
    ] {
        let mut args = vec!["list"];
        args.extend(options);
        let result = list(directory.path(), &args);
        assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
        assert!(result.stderr.is_empty());
        assert_eq!(
            result.stdout,
            concat!(
                "├── DIR2\\   first\n",
                "│   │      \n",
                "│   │       tail \n",
                "│   │      \n",
                "│   │      \n",
                "│   ├── middle\\\n",
                "│   │   └── deep\\\n",
                "│   │       └── lost  literal\\n\n",
                "│   └── orphan  missing\n",
                "├── dir10\\\n",
                "│   └── file  last\n",
                "├── empty  \n",
                "├── file2  two\n",
                "└── file10  ten\n",
            )
            .as_bytes()
        );
    }
    for (path, bytes) in fixtures {
        assert_eq!(
            fs::read(directory.path().join(path)).unwrap(),
            bytes.as_bytes()
        );
    }
}

#[test]
fn tree_json_is_the_ordinary_recursive_list_without_supporting_nodes() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("child/middle/deep")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}CHILD own\r\nz parent",
    )
    .unwrap();
    fs::write(
        directory.path().join("child/middle/deep/descript.ion"),
        "\u{feff}empty \r\nother unknown\\n\u{4}foreign",
    )
    .unwrap();
    let recursive = list(directory.path(), &["list", "-r", "--json"]);
    assert_eq!(recursive.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&recursive.stdout).unwrap(),
        serde_json::json!({"entries": [
            {"name":"CHILD", "comment":"own", "extension":"none"},
            {"name":"z", "comment":"parent", "extension":"none"},
            {"name":"child\\middle\\deep\\empty", "comment":"", "extension":"none"},
            {"name":"child\\middle\\deep\\other", "comment":"unknown\\n", "extension":"unknown"},
        ], "errors": []})
    );
    for options in [vec!["--tree"], vec!["--tree", "-r"]] {
        let mut args = vec!["--json", "list"];
        args.extend(options);
        let result = list(directory.path(), &args);
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stderr.is_empty());
        assert_eq!(result.stdout, recursive.stdout);
    }
}

#[test]
fn tree_aligns_wide_names_under_last_branches_without_wrapping() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("中😀")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}中😀 dir\\nend\u{4}\u{c2}",
    )
    .unwrap();
    fs::write(
        directory.path().join("中😀/descript.ion"),
        format!("\u{feff}Ａ  first\t\\n\\n{}\\n\u{4}\u{c2}", "x".repeat(300)),
    )
    .unwrap();
    let result = list(directory.path(), &["list", "--tree"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        format!(
            "└── 中😀\\  dir\n    │      end\n    └── Ａ   first\t\n            \n            {}\n            \n",
            "x".repeat(300)
        )
    );
}

#[test]
fn tree_and_long_are_mutually_exclusive_in_text_and_json() {
    let directory = tempfile::tempdir().unwrap();
    for options in [
        vec!["--tree", "--long"],
        vec!["-l", "--tree"],
        vec!["--tree", "-r", "-l"],
    ] {
        for json in [false, true] {
            let mut args = vec!["list"];
            args.extend(options.iter().copied());
            if json {
                args.push("--json");
            }
            let result = list(directory.path(), &args);
            assert_eq!(result.status.code(), Some(2));
            assert!(result.stdout.is_empty());
            if json {
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["error"]["code"],
                    "invalid_argument"
                );
            } else {
                assert!(
                    String::from_utf8(result.stderr)
                        .unwrap()
                        .contains("invalid_argument")
                );
            }
        }
    }
}

#[test]
fn tree_without_any_records_emits_nothing_even_with_unrelated_directories() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("child/deep")).unwrap();
    fs::write(directory.path().join("child/descript.ion"), "\u{feff}\r\n").unwrap();
    let result = list(directory.path(), &["list", "--tree"]);
    assert_eq!(result.status.code(), Some(0));
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
}

#[test]
fn recursive_list_emits_parent_records_then_naturally_ordered_subtrees_in_all_modes() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("dir2/middle/deep")).unwrap();
    fs::create_dir(directory.path().join("dir10")).unwrap();
    let fixtures = [
        (
            "descript.ion",
            "\u{feff}z parent\r\nDIR2 directory\r\nempty ",
        ),
        (
            "dir2/descript.ion",
            "\u{feff}orphan first\\nsecond\u{4}\u{c2}",
        ),
        (
            "dir2/middle/deep/descript.ion",
            "\u{feff}lost literal\\n\u{4}foreign",
        ),
        ("dir10/descript.ion", "\u{feff}file last"),
    ];
    for (path, bytes) in fixtures {
        fs::write(directory.path().join(path), bytes).unwrap();
    }
    let expected = serde_json::json!({"entries": [
        {"name":"DIR2", "comment":"directory", "extension":"none"},
        {"name":"empty", "comment":"", "extension":"none"},
        {"name":"z", "comment":"parent", "extension":"none"},
        {"name":"dir2\\orphan", "comment":"first\nsecond", "extension":"tc"},
        {"name":"dir2\\middle\\deep\\lost", "comment":"literal\\n", "extension":"unknown"},
        {"name":"dir10\\file", "comment":"last", "extension":"none"},
    ], "errors": []});
    for option in ["--recursive", "-r"] {
        for long in [false, true] {
            let mut args = vec!["list", option, "--json"];
            if long {
                args.push("-l");
            }
            let result = list(directory.path(), &args);
            assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
                expected
            );
        }
    }
    let result = list(directory.path(), &["list", "-r"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        result.stdout,
        concat!(
            "DIR2\\                  directory\n",
            "empty                  \n",
            "z                      parent\n",
            "dir2\\orphan            first\n",
            "                       second\n",
            "dir2\\middle\\deep\\lost  literal\\n\n",
            "dir10\\file             last\n",
        )
        .as_bytes()
    );
    let result = list(directory.path(), &["list", "--recursive", "--long"]);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(
        result.stdout,
        concat!(
            "DIR2\\\n    directory\n\nempty\n    \n\nz\n    parent\n\n",
            "dir2\\orphan\n    first\n    second\n\n",
            "dir2\\middle\\deep\\lost\n    literal\\n\n\n",
            "dir10\\file\n    last\n\n",
        )
        .as_bytes()
    );
    for (path, bytes) in fixtures {
        assert_eq!(
            fs::read(directory.path().join(path)).unwrap(),
            bytes.as_bytes()
        );
    }
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
fn recursive_description_errors_preserve_other_records_in_every_mode() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("dir2")).unwrap();
    fs::create_dir(directory.path().join("dir10")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfparent ok",
    )
    .unwrap();
    fs::write(
        directory.path().join("dir2/descript.ion"),
        b"\xef\xbb\xbfchild ok",
    )
    .unwrap();
    let file = directory.path().join("dir10/descript.ion");
    for (bytes, code, line) in [
        (b"missing BOM".as_slice(), "invalid_encoding", None),
        (
            b"\xef\xbb\xbfok body\n\"unclosed".as_slice(),
            "invalid_format",
            Some(2),
        ),
    ] {
        fs::write(&file, bytes).unwrap();
        for options in [
            vec![],
            vec!["-l"],
            vec!["--json"],
            vec!["-l", "--json"],
            vec!["--tree"],
            vec!["--tree", "--json"],
        ] {
            let mut args = vec!["list", "-r"];
            args.extend(options);
            let result = list(directory.path(), &args);
            assert_eq!(result.status.code(), Some(1));
            assert!(result.stderr.is_empty());
            if args.contains(&"--json") {
                let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
                assert_eq!(value["entries"][0]["name"], "parent");
                assert_eq!(value["entries"][1]["name"], "dir2\\child");
                assert_eq!(
                    value["entries"].as_array().unwrap().len(),
                    if line.is_some() { 3 } else { 2 }
                );
                assert_eq!(value["errors"].as_array().unwrap().len(), 1);
                assert_eq!(value["errors"][0]["code"], code);
                assert_eq!(value["errors"][0]["file"], ".\\dir10\\descript.ion");
                assert_eq!(value["errors"][0]["line"], serde_json::json!(line));
            } else {
                let text = String::from_utf8(result.stdout).unwrap();
                assert!(text.find("parent").unwrap() < text.find("错误（1）：").unwrap());
                assert!(text.contains("child"));
                assert!(text.contains(code));
                assert!(text.contains("dir10\\descript.ion"));
            }
        }
        assert_eq!(fs::read(&file).unwrap(), bytes);
    }
}

#[cfg(windows)]
#[test]
fn an_inaccessible_child_is_skipped_and_other_directories_are_visited() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfparent ok",
    )
    .unwrap();
    let child = directory.path().join("child");
    fs::create_dir(&child).unwrap();
    fs::write(child.join("descript.ion"), b"\xef\xbb\xbfhidden skipped").unwrap();
    fs::create_dir(directory.path().join("later")).unwrap();
    fs::write(
        directory.path().join("later/descript.ion"),
        b"\xef\xbb\xbfafter visited",
    )
    .unwrap();
    // Deny directory enumeration, while leaving file reads and ACL restoration allowed.
    let script = "$ErrorActionPreference='Stop'; $p=$env:DION_TEST_DIRECTORY; $acl=Get-Acl -LiteralPath $p; $rule=[Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.WindowsIdentity]::GetCurrent().User, 'ListDirectory', 'Deny'); $acl.AddAccessRule($rule); Set-Acl -LiteralPath $p -AclObject $acl";
    let acl = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env_remove("PSModulePath")
        .env("DION_TEST_DIRECTORY", &child)
        .output()
        .unwrap();
    assert!(acl.status.success(), "{:?}", acl.stderr);
    let results: Vec<_> = [
        vec!["list", "-r"],
        vec!["list", "-r", "-l"],
        vec!["list", "-r", "--json"],
        vec!["list", "--tree"],
        vec!["list", "--tree", "--json"],
    ]
    .iter()
    .map(|args| list(directory.path(), args))
    .collect();
    let start = list(directory.path(), &["list", "child", "--json"]);
    let restore = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script.replace("AddAccessRule", "RemoveAccessRule"),
        ])
        .env_remove("PSModulePath")
        .env("DION_TEST_DIRECTORY", &child)
        .output()
        .unwrap();
    assert!(restore.status.success(), "{:?}", restore.stderr);
    assert_eq!(start.status.code(), Some(1));
    assert!(start.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&start.stderr).unwrap()["error"]["code"],
        "io_error"
    );
    for result in results {
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stderr.is_empty());
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(text.contains("parent"));
        assert!(text.contains("after"));
        assert!(text.contains("io_error"));
        assert!(!text.contains("hidden"));
    }
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
    assert!(result.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["entries"], serde_json::json!([]));
    assert_eq!(value["errors"][0]["code"], "invalid_encoding");
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
        vec!["list", "-r"],
        vec!["list", "-r", "--long"],
        vec!["list", "-r", "--json"],
        vec!["list", "--tree"],
        vec!["list", "--tree", "--json"],
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
        ], "errors": []})
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
        assert_eq!(result.stdout, b"{\"entries\":[],\"errors\":[]}\n");
        assert!(result.stderr.is_empty());
        let result = list(directory.path(), &["list"]);
        assert_eq!(result.status.code(), Some(0));
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn bad_records_are_skipped_without_losing_records_before_or_after_them() {
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
        bytes.extend(b"\r\nthird later");
        fs::write(directory.path().join("descript.ion"), &bytes).unwrap();
        for args in [
            vec!["list"],
            vec!["list", "--long"],
            vec!["list", "--json"],
            vec!["list", "-l", "--json"],
            vec!["list", "--tree"],
            vec!["list", "--tree", "--json"],
        ] {
            let result = list(directory.path(), &args);
            assert_eq!(result.status.code(), Some(1));
            assert!(result.stderr.is_empty());
            if args.contains(&"--json") {
                let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
                assert_eq!(
                    value["entries"],
                    serde_json::json!([
                        {"name":"first", "comment":"ok", "extension":"none"},
                        {"name":"second", "comment":"multiline\nbody", "extension":"tc"},
                        {"name":"third", "comment":"later", "extension":"none"},
                    ])
                );
                assert_eq!(value["errors"].as_array().unwrap().len(), 1);
                assert_eq!(value["errors"][0]["code"], code);
                assert_eq!(value["errors"][0]["file"], ".\\descript.ion");
                assert_eq!(value["errors"][0]["line"], 4);
            } else {
                let error = String::from_utf8(result.stdout).unwrap();
                assert!(error.find("third").unwrap() < error.find("错误（1）：").unwrap());
                assert!(error.contains(code));
                assert!(error.contains("descript.ion"));
                assert!(error.contains("第 4 行"));
            }
        }
        assert_eq!(
            fs::read(directory.path().join("descript.ion")).unwrap(),
            bytes
        );
    }
}

#[test]
fn invalid_starting_directories_are_fatal_errors() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("file"), b"not a directory").unwrap();
    for path in ["missing", "file"] {
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
fn multiple_bad_lines_are_reported_in_physical_order_after_the_list() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = b"\xef\xbb\xbf\r\n\"unclosed\rfirst kept\nFIRST duplicate\r\nbad \xff\n\"\" empty\n\"quoted\"tail\nafter readable";
    let file = directory.path().join("descript.ion");
    fs::write(&file, bytes).unwrap();
    let expected_codes = [
        "invalid_format",
        "invalid_format",
        "invalid_encoding",
        "invalid_format",
        "invalid_format",
    ];
    for options in [
        vec![],
        vec!["-l"],
        vec!["--tree"],
        vec!["--json"],
        vec!["-l", "--json"],
        vec!["--tree", "--json"],
    ] {
        let mut args = vec!["list"];
        args.extend(options);
        let result = list(directory.path(), &args);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stderr.is_empty());
        if args.contains(&"--json") {
            let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
            assert_eq!(
                value["entries"],
                serde_json::json!([
                    {"name":"after", "comment":"readable", "extension":"none"},
                    {"name":"first", "comment":"kept", "extension":"none"},
                ])
            );
            let errors = value["errors"].as_array().unwrap();
            assert_eq!(errors.len(), 5);
            for (error, (code, line)) in errors
                .iter()
                .zip(expected_codes.into_iter().zip([2, 4, 5, 6, 7]))
            {
                assert_eq!(error["code"], code);
                assert_eq!(error["line"], line);
                assert_eq!(error["file"], ".\\descript.ion");
            }
        } else {
            let text = String::from_utf8(result.stdout).unwrap();
            let (entries, errors) = text.split_once("错误（5）：\n").unwrap();
            assert!(entries.contains("readable"));
            assert!(entries.contains("kept"));
            assert!(!entries.contains("duplicate"));
            let lines: Vec<_> = errors.lines().collect();
            assert_eq!(lines.len(), 5);
            for (error, (code, line)) in lines
                .iter()
                .zip(expected_codes.into_iter().zip([2, 4, 5, 6, 7]))
            {
                assert!(error.starts_with(code));
                assert!(error.ends_with(&format!(" 第 {line} 行")));
            }
        }
        assert_eq!(fs::read(&file).unwrap(), bytes);
    }
}

#[test]
fn skipped_description_files_do_not_hide_descendants_and_errors_follow_traversal_order() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("dir2/deep")).unwrap();
    fs::create_dir_all(directory.path().join("dir10/descript.ion")).unwrap();
    fs::create_dir(directory.path().join("dir10/deep")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfparent kept",
    )
    .unwrap();
    fs::write(directory.path().join("dir2/descript.ion"), b"missing BOM").unwrap();
    fs::write(
        directory.path().join("dir2/deep/descript.ion"),
        b"\xef\xbb\xbfchild kept\n\"unclosed",
    )
    .unwrap();
    fs::write(
        directory.path().join("dir10/deep/descript.ion"),
        b"\xef\xbb\xbfafter kept",
    )
    .unwrap();
    let recursive = list(directory.path(), &["list", "-r", "--json"]);
    assert_eq!(recursive.status.code(), Some(1));
    assert!(recursive.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&recursive.stdout).unwrap();
    assert_eq!(
        value["entries"],
        serde_json::json!([
            {"name":"parent", "comment":"kept", "extension":"none"},
            {"name":"dir2\\deep\\child", "comment":"kept", "extension":"none"},
            {"name":"dir10\\deep\\after", "comment":"kept", "extension":"none"},
        ])
    );
    let errors = value["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 3);
    assert_eq!(errors[0]["code"], "invalid_encoding");
    assert_eq!(errors[0]["file"], ".\\dir2\\descript.ion");
    assert!(errors[0].get("line").is_none());
    assert_eq!(errors[1]["code"], "invalid_format");
    assert_eq!(errors[1]["line"], 2);
    assert_eq!(errors[2]["code"], "io_error");
    assert_eq!(errors[2]["file"], ".\\dir10\\descript.ion");
    let tree = list(directory.path(), &["list", "--tree", "--json"]);
    assert_eq!(tree.status.code(), Some(1));
    assert_eq!(tree.stdout, recursive.stdout);
    let direct = list(directory.path(), &["list", "dir10", "--json"]);
    assert_eq!(direct.status.code(), Some(1));
    assert!(direct.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&direct.stdout).unwrap();
    assert_eq!(value["entries"], serde_json::json!([]));
    assert_eq!(value["errors"][0]["code"], "io_error");
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
        vec!["list", "--recursive", "-r", "--json"],
        vec!["get", "a", "-r", "--json"],
        vec!["set", "a", "body", "--recursive", "--json"],
        vec!["remove", "a", "-r", "--json"],
        vec!["list", "--tree", "--tree", "--json"],
        vec!["list", "--color", "--json"],
        vec!["list", "--color", "loud", "--json"],
        vec!["list", "--color=always", "--json"],
        vec!["list", "--color", "always", "--color", "never", "--json"],
        vec!["get", "a", "--tree", "--json"],
        vec!["get", "a", "--color", "auto", "--json"],
        vec!["set", "a", "body", "--tree", "--json"],
        vec!["set", "a", "body", "--color", "auto", "--json"],
        vec!["remove", "a", "--tree", "--json"],
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
    assert_eq!(result.stdout, b"{\"entries\":[],\"errors\":[]}\n");
    assert!(result.stderr.is_empty());
}

#[cfg(windows)]
#[test]
fn locked_description_is_reported_and_descendants_are_still_visited() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbffirst ok",
    )
    .unwrap();
    fs::create_dir(directory.path().join("child")).unwrap();
    fs::write(
        directory.path().join("child/descript.ion"),
        b"\xef\xbb\xbfafter visited",
    )
    .unwrap();
    let _locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(directory.path().join("descript.ion"))
        .unwrap();
    let result = list(directory.path(), &["list", "-r", "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["entries"].as_array().unwrap().len(), 1);
    assert_eq!(value["entries"][0]["name"], "child\\after");
    assert_eq!(value["errors"][0]["code"], "io_error");
}

#[test]
fn tree_connects_multiline_directory_comment_to_children_and_maintains_alignment() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("200 Note/child_dir")).unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        "\u{feff}\"200 Note\" line1\\nline2\\nline3\u{4}\u{c2}\r\nother.txt other_note",
    )
    .unwrap();
    fs::write(
        directory.path().join("200 Note/descript.ion"),
        "\u{feff}child_dir child_note\r\nchild_file.txt file_note",
    )
    .unwrap();

    let result = list(directory.path(), &["list", "--tree"]);
    assert_eq!(result.status.code(), Some(0));
    let expected = concat!(
        "├── 200 Note\\  line1\n",
        "│   │          line2\n",
        "│   │          line3\n",
        "│   ├── child_dir\\  child_note\n",
        "│   └── child_file.txt  file_note\n",
        "└── other.txt  other_note\n",
    );
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);

    let colored = list(directory.path(), &["list", "--tree", "--color", "always"]);
    assert_eq!(colored.status.code(), Some(0));
    let expected_colored = concat!(
        "\u{1b}[36m├──\u{1b}[39m \u{1b}[36m200 Note\\\u{1b}[39m  line1\n",
        "\u{1b}[36m│\u{1b}[39m   \u{1b}[36m│\u{1b}[39m          line2\n",
        "\u{1b}[36m│\u{1b}[39m   \u{1b}[36m│\u{1b}[39m          line3\n",
        "\u{1b}[36m│\u{1b}[39m   \u{1b}[36m├──\u{1b}[39m \u{1b}[36mchild_dir\\\u{1b}[39m  child_note\n",
        "\u{1b}[36m│\u{1b}[39m   \u{1b}[36m└──\u{1b}[39m \u{1b}[36mchild_file.txt\u{1b}[39m  file_note\n",
        "\u{1b}[36m└──\u{1b}[39m \u{1b}[36mother.txt\u{1b}[39m  other_note\n",
    );
    assert_eq!(String::from_utf8(colored.stdout).unwrap(), expected_colored);
}
