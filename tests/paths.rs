use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

// Shared entry-path cases for get/set and, when implemented, remove.
const DIRECTORY_PATHS: [&str; 4] = ["folder", "folder/", "folder/.", "./folder"];

fn run(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
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
