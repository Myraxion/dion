use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

// Shared entry-path cases for get/set/remove.
const DIRECTORY_PATHS: [&str; 4] = ["folder", "folder/", "folder/.", "./folder"];

fn run(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn remove_locates_directory_comments_in_the_parent_for_all_path_spellings() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("folder");
    fs::create_dir(&folder).unwrap();
    let parent_file = directory.path().join("descript.ion");
    let inner_file = folder.join("descript.ion");
    let inner_bytes = b"\xef\xbb\xbffolder inner comment";
    fs::write(&inner_file, inner_bytes).unwrap();
    let absolute = folder.to_str().unwrap();
    for (cwd, path) in DIRECTORY_PATHS
        .iter()
        .map(|path| (directory.path(), *path))
        .chain([
            (directory.path(), absolute),
            (folder.as_path(), "."),
            (folder.as_path(), "../folder"),
        ])
    {
        fs::write(&parent_file, b"\xef\xbb\xbffolder body\nother body\n").unwrap();
        let result = run(cwd, &["remove", path]);
        assert_eq!(result.status.code(), Some(0), "{path}: {:?}", result.stderr);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        assert_eq!(fs::read(&parent_file).unwrap(), b"\xef\xbb\xbfother body\n");
        assert_eq!(fs::read(&inner_file).unwrap(), inner_bytes);
    }
}

#[test]
fn get_and_set_locate_directory_comments_in_the_parent_for_all_path_spellings() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("folder");
    fs::create_dir(&folder).unwrap();
    let parent_file = directory.path().join("descript.ion");
    fs::write(&parent_file, b"\xef\xbb\xbf\r\nfolder original\r\n").unwrap();
    let inner_file = folder.join("descript.ion");
    let inner_bytes = b"\xef\xbb\xbffolder inner comment";
    fs::write(&inner_file, inner_bytes).unwrap();

    let absolute = folder.to_str().unwrap();
    let cases = DIRECTORY_PATHS
        .iter()
        .map(|path| (directory.path(), *path))
        .chain([
            (directory.path(), absolute),
            (folder.as_path(), "."),
            (folder.as_path(), "../folder"),
        ]);
    let mut expected = "original";
    for (cwd, path) in cases {
        let result = run(cwd, &["get", path]);
        assert_eq!(
            result.status.code(),
            Some(0),
            "{path}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, expected.as_bytes(), "{path}");
        assert!(result.stderr.is_empty());

        let body = if expected == "original" {
            "updated"
        } else {
            "original"
        };
        let result = run(cwd, &["set", path, body]);
        assert_eq!(
            result.status.code(),
            Some(0),
            "{path}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        assert_eq!(
            fs::read(&parent_file).unwrap(),
            format!("\u{feff}\r\nfolder {body}\r\n").as_bytes()
        );
        assert_eq!(fs::read(&inner_file).unwrap(), inner_bytes);
        expected = body;
    }
}

#[cfg(windows)]
#[test]
fn extended_directory_dot_path_operates_on_the_directory_entry() {
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("folder");
    fs::create_dir(&folder).unwrap();
    let extended = fs::canonicalize(&folder).unwrap();
    let path = format!("{}\\.", extended.display());
    let parent_file = directory.path().join("descript.ion");
    let inner_file = folder.join("descript.ion");
    fs::write(&parent_file, b"\xef\xbb\xbffolder own\nother untouched\r").unwrap();
    fs::write(&inner_file, b"\xef\xbb\xbfchild inner\n").unwrap();

    let result = run(directory.path(), &["get", &path]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    assert_eq!(result.stdout, b"own");
    let result = run(directory.path(), &["--json", "set", &path, "updated"]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    assert_eq!(result.stdout, b"{\"changed\":true}\n");
    assert_eq!(
        fs::read(&parent_file).unwrap(),
        b"\xef\xbb\xbffolder updated\r\nother untouched\r"
    );
    let result = run(directory.path(), &["list", &path]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    assert_eq!(result.stdout, b"child  inner\n");
    let result = run(directory.path(), &["remove", &path]);
    assert_eq!(result.status.code(), Some(0), "{:?}", result.stderr);
    assert_eq!(
        fs::read(&parent_file).unwrap(),
        b"\xef\xbb\xbfother untouched\r"
    );
    assert_eq!(fs::read(&inner_file).unwrap(), b"\xef\xbb\xbfchild inner\n");
}

#[cfg(windows)]
fn success(output: &Output) {
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
}

#[cfg(windows)]
#[test]
fn all_commands_support_long_unicode_paths_and_orphan_records() {
    use std::os::windows::fs::MetadataExt;
    let directory = tempfile::tempdir().unwrap();
    let relative = format!(
        "{}\\{}\\{}\\资料 😀",
        "a".repeat(100),
        "b".repeat(100),
        "c".repeat(100)
    );
    let folder = directory.path().join(&relative);
    fs::create_dir_all(&folder).unwrap();
    let extended = fs::canonicalize(&folder).unwrap();
    assert!(folder.as_os_str().len() > 260);
    let file = folder.join("descript.ion");
    let entry = folder.join("照片 😀.txt");
    fs::write(&entry, b"entry").unwrap();
    let paths = [
        relative,
        folder.to_str().unwrap().to_owned(),
        extended.to_str().unwrap().to_owned(),
    ];
    for path in paths {
        let entry_path = format!("{path}\\照片 😀.txt");
        success(&run(directory.path(), &["set", &entry_path, "中文"]));
        assert_eq!(
            fs::read(&file).unwrap(),
            "\u{feff}\r\n\"照片 😀.txt\" 中文\r\n".as_bytes()
        );
        assert_ne!(fs::metadata(&file).unwrap().file_attributes() & 2, 0);
        let created = fs::metadata(&file).unwrap().creation_time();
        let result = run(directory.path(), &["get", &entry_path]);
        success(&result);
        assert_eq!(result.stdout, "中文".as_bytes());
        let result = run(directory.path(), &["--json", "get", &entry_path]);
        success(&result);
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"name":"照片 😀.txt", "comment":"中文", "extension":"none"})
        );
        let result = run(directory.path(), &["--json", "set", &entry_path, "中文"]);
        success(&result);
        assert_eq!(result.stdout, b"{\"changed\":false}\n");

        let original = "\u{feff}\"照片 😀.txt\" 中文\r\nmissing orphan\nkeep untouched\r";
        fs::write(&file, original.as_bytes()).unwrap();
        let result = run(directory.path(), &["--json", "set", &entry_path, "更新"]);
        success(&result);
        assert_eq!(result.stdout, b"{\"changed\":true}\n");
        assert_eq!(fs::metadata(&file).unwrap().creation_time(), created);
        assert_ne!(fs::metadata(&file).unwrap().file_attributes() & 2, 0);
        assert_eq!(
            fs::read(&file).unwrap(),
            "\u{feff}\"照片 😀.txt\" 更新\r\nmissing orphan\nkeep untouched\r".as_bytes()
        );
        let result = run(directory.path(), &["list", &path]);
        success(&result);
        assert_eq!(
            result.stdout,
            "keep         untouched\nmissing      orphan\n照片 😀.txt  更新\n".as_bytes()
        );
        let result = run(directory.path(), &["--json", "list", &path]);
        success(&result);
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(value["entries"].as_array().unwrap().len(), 3);

        let orphan = format!("{path}\\missing");
        let result = run(directory.path(), &["get", &orphan]);
        success(&result);
        assert_eq!(result.stdout, b"orphan");
        let result = run(directory.path(), &["set", &orphan, "no"]);
        assert_eq!(result.status.code(), Some(1));
        let result = run(directory.path(), &["--json", "remove", &orphan]);
        success(&result);
        assert_eq!(result.stdout, b"{\"changed\":true}\n");
        assert_eq!(
            fs::read(&file).unwrap(),
            "\u{feff}\"照片 😀.txt\" 更新\r\nkeep untouched\r".as_bytes()
        );
        success(&run(
            directory.path(),
            &["remove", &format!("{path}\\keep")],
        ));
        success(&run(directory.path(), &["remove", &entry_path]));
        assert!(!file.exists());
        let result = run(directory.path(), &["--json", "remove", &entry_path]);
        success(&result);
        assert_eq!(result.stdout, b"{\"changed\":false}\n");
        let result = run(directory.path(), &["--json", "get", &entry_path]);
        assert_eq!(result.status.code(), Some(3));
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["error"]["code"], "not_found");
    }
}

#[cfg(windows)]
#[test]
fn roots_reject_entry_commands_before_accessing_disk() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory
        .path()
        .ancestors()
        .last()
        .unwrap()
        .to_str()
        .unwrap();
    let disk_roots = [
        root.to_owned(),
        format!("{root}."),
        format!("\\\\?\\{root}"),
        format!("\\\\?\\{root}.."),
    ];
    for path in disk_roots.iter().map(String::as_str) {
        for command in ["get", "set", "remove"] {
            for json in [false, true] {
                let mut args = vec![command, path];
                if command == "set" {
                    args.push("body");
                }
                if json {
                    args.push("--json");
                }
                let result = run(directory.path(), &args);
                assert_eq!(
                    result.status.code(),
                    Some(2),
                    "{command} {path}: {:?}",
                    result.stderr
                );
                assert!(result.stdout.is_empty());
                if json {
                    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
                    assert_eq!(error["error"]["code"], "invalid_argument");
                }
            }
        }
    }
    for path in disk_roots {
        let result = run(directory.path(), &["--json", "list", &path]);
        success(&result);
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert!(value["entries"].is_array());
    }
}

#[cfg(windows)]
fn check_link_entry(directory: &Path, link: &Path, target_parent: &Path, is_dir: bool) {
    let file = directory.join("descript.ion");
    let target_file = target_parent.join("descript.ion");
    let target_bytes = b"\xef\xbb\xbftarget destination comment\n";
    fs::write(&target_file, target_bytes).unwrap();
    fs::write(&file, b"\xef\xbb\xbflink own comment\nother untouched\r").unwrap();
    let path = link.to_str().unwrap();
    let result = run(directory, &["get", path]);
    success(&result);
    assert_eq!(result.stdout, b"own comment");
    let result = run(directory, &["--json", "set", path, "updated"]);
    success(&result);
    assert_eq!(result.stdout, b"{\"changed\":true}\n");
    assert_eq!(
        fs::read(&file).unwrap(),
        b"\xef\xbb\xbflink updated\r\nother untouched\r"
    );
    let result = run(directory, &["--json", "list", directory.to_str().unwrap()]);
    success(&result);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["entries"][0]["comment"], "updated");
    let result = run(directory, &["list", directory.to_str().unwrap()]);
    success(&result);
    assert_eq!(
        result.stdout,
        if is_dir {
            b"link\\  updated\nother  untouched\n".as_slice()
        } else {
            b"link   updated\nother  untouched\n".as_slice()
        }
    );
    let result = run(directory, &["list", directory.to_str().unwrap(), "-l"]);
    success(&result);
    assert_eq!(
        result.stdout,
        if is_dir {
            b"link\\\n    updated\n\nother\n    untouched\n\n".as_slice()
        } else {
            b"link\n    updated\n\nother\n    untouched\n\n".as_slice()
        }
    );
    if is_dir {
        let inner = link.join("descript.ion");
        fs::write(&inner, b"\xef\xbb\xbfchild inside\n").unwrap();
        let result = run(directory, &["list", path]);
        success(&result);
        assert_eq!(result.stdout, b"child  inside\n");
        assert_eq!(fs::read(&inner).unwrap(), b"\xef\xbb\xbfchild inside\n");
        fs::create_dir(link.join("nested")).unwrap();
        fs::write(link.join("nested/descript.ion"), b"\xef\xbb\xbfdeep nested").unwrap();
        for options in [vec!["-r"], vec!["--recursive", "-l"], vec!["-r", "--json"]] {
            let mut args = vec!["list"];
            args.extend(options.iter().copied());
            let result = run(directory, &args);
            success(&result);
            assert!(!String::from_utf8_lossy(&result.stdout).contains("inside"));
            assert!(!String::from_utf8_lossy(&result.stdout).contains("nested"));
            if !options.contains(&"--json") {
                assert!(String::from_utf8_lossy(&result.stdout).starts_with("link\\"));
            }
            args.insert(1, path);
            let result = run(directory, &args);
            success(&result);
            if options.contains(&"--json") {
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
                    serde_json::json!({"entries":[
                        {"name":"child", "comment":"inside", "extension":"none"},
                        {"name":"nested\\deep", "comment":"nested", "extension":"none"},
                    ]})
                );
            } else {
                let text = String::from_utf8(result.stdout).unwrap();
                assert!(text.contains("inside"));
                assert!(text.contains("nested\\deep"));
            }
        }
    }
    success(&run(directory, &["remove", path]));
    assert_eq!(fs::read(&file).unwrap(), b"\xef\xbb\xbfother untouched\r");
    assert_eq!(fs::read(&target_file).unwrap(), target_bytes);
}

#[test]
fn recursive_legacy_names_remain_complete_and_never_visit_paths_named_by_records() {
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("descript.ion"), b"invalid encoding").unwrap();
    fs::create_dir_all(directory.path().join("child/nested")).unwrap();
    let names = [
        ".",
        "..",
        "../outside",
        "nested\\",
        "nested/path",
        "nested\\path",
        outside.path().to_str().unwrap(),
    ];
    let bytes = format!(
        "\u{feff}{}",
        names
            .iter()
            .map(|name| format!("\"{name}\" body\r\n"))
            .collect::<String>()
    );
    fs::write(directory.path().join("child/descript.ion"), &bytes).unwrap();
    let result = run(directory.path(), &["list", "-r", "--json"]);
    success(&result);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let entries = value["entries"].as_array().unwrap();
    assert_eq!(entries.len(), names.len());
    for name in names {
        assert!(
            entries
                .iter()
                .any(|entry| entry["name"] == format!("child\\{name}"))
        );
    }
    let result = run(directory.path(), &["list", "-r", "-l"]);
    success(&result);
    let text = String::from_utf8(result.stdout).unwrap();
    for name in names {
        assert!(text.contains(&format!("child\\{name}\n    body\n\n")));
    }
    assert_eq!(
        fs::read(directory.path().join("child/descript.ion")).unwrap(),
        bytes.as_bytes()
    );
}

#[cfg(windows)]
#[test]
fn file_and_directory_symlinks_keep_comments_at_the_input_entry() {
    use std::os::windows::fs::{symlink_dir, symlink_file};
    for is_dir in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let targets = tempfile::tempdir().unwrap();
        let target = targets.path().join("target");
        if is_dir {
            fs::create_dir(&target).unwrap();
        } else {
            fs::write(&target, b"target").unwrap();
        }
        let link = directory.path().join("link");
        let result = if is_dir {
            symlink_dir(&target, &link)
        } else {
            symlink_file(&target, &link)
        };
        if let Err(error) = result {
            if error.raw_os_error() == Some(1314) {
                eprintln!(
                    "UNVERIFIED: {} symlink creation requires privilege: {error}",
                    if is_dir { "directory" } else { "file" }
                );
                continue;
            }
            panic!("symlink creation failed: {error}");
        }
        check_link_entry(directory.path(), &link, targets.path(), is_dir);
        if is_dir {
            fs::remove_dir(&link).unwrap();
        } else {
            fs::remove_file(&link).unwrap();
        }
    }
}

#[cfg(windows)]
#[test]
fn directory_junction_keeps_comments_at_the_input_entry() {
    let directory = tempfile::tempdir().unwrap();
    let targets = tempfile::tempdir().unwrap();
    let target = targets.path().join("target");
    fs::create_dir(&target).unwrap();
    let link = directory.path().join("link");
    let script = format!(
        "New-Item -ItemType Junction -Path '{}' -Target '{}' | Out-Null",
        link.display().to_string().replace('\'', "''"),
        target.display().to_string().replace('\'', "''")
    );
    let result = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .env_remove("PSModulePath")
        .output()
        .unwrap();
    success(&result);
    check_link_entry(directory.path(), &link, targets.path(), true);
    fs::remove_dir(&link).unwrap();
}
