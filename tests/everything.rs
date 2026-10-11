#![cfg(windows)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Output},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn peer_exe() -> &'static Path {
    static PEER: OnceLock<(TempDir, PathBuf)> = OnceLock::new();
    &PEER
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let exe = dir.path().join("controlled everything.exe");
            let output = Command::new("rustc")
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/everything.rs"))
                .args(["--edition=2024", "-o"])
                .arg(&exe)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            (dir, exe)
        })
        .1
}

struct Peer {
    child: Child,
    dir: TempDir,
    instance: String,
}
impl Peer {
    fn start(instance: &str, response: &[u8], mode: &str) -> Option<Self> {
        let dir = tempfile::tempdir().unwrap();
        let response_file = dir.path().join("response.bin");
        let ready = dir.path().join("ready.txt");
        fs::write(&response_file, response).unwrap();
        let child = Command::new(peer_exe())
            .arg(instance)
            .arg(response_file)
            .arg(&ready)
            .arg(dir.path().join("query.txt"))
            .arg(mode)
            .spawn()
            .unwrap();
        let mut peer = Self {
            child,
            dir,
            instance: instance.into(),
        };
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Ok(state) = fs::read_to_string(&ready) {
                match state.as_str() {
                    "ready" => return Some(peer),
                    "occupied" => return None,
                    // File creation can become visible before its contents are written.
                    _ => {}
                }
            }
            assert!(
                peer.child.try_wait().unwrap().is_none(),
                "IPC peer exited before becoming ready"
            );
            assert!(Instant::now() < deadline, "IPC peer did not become ready");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn new(response: &[u8], mode: &str) -> Self {
        let id = tempfile::tempdir().unwrap();
        Self::start(
            &format!(
                "dion-test-{}",
                id.path().file_name().unwrap().to_str().unwrap()
            ),
            response,
            mode,
        )
        .unwrap()
    }

    fn list(&self, directory: &Path, options: &[&str]) -> Output {
        let mut command = cli(directory);
        command
            .args(["--everything", "--everything-instance", &self.instance])
            .args(options);
        command.output().unwrap()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn cli(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dion"));
    command
        .arg("list")
        .arg(directory)
        .args(["--lang", "en", "--color", "never"]);
    command
}

// LISTW consists of seven DWORDs, followed by 12-byte ITEMW records and UTF-16 strings.
// Offsets refer to the start of LISTW, as specified by the public IPC protocol.
fn response(paths: &[PathBuf]) -> Vec<u8> {
    let mut bytes = vec![0; 28 + 12 * paths.len()];
    for offset in [4, 8, 16, 20] {
        bytes[offset..offset + 4].copy_from_slice(&(paths.len() as u32).to_le_bytes());
    }
    for (index, path) in paths.iter().enumerate() {
        for (offset, value) in [
            (4, path.file_name().unwrap()),
            (8, path.parent().unwrap().as_os_str()),
        ] {
            use std::os::windows::ffi::OsStrExt;
            let position = 28 + 12 * index + offset;
            let string_offset = bytes.len() as u32;
            bytes[position..position + 4].copy_from_slice(&string_offset.to_le_bytes());
            for unit in value.encode_wide().chain(Some(0)) {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
        }
    }
    bytes
}

fn success(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn waits_for_ready_file_contents_before_querying_the_peer() {
    let directory = tempfile::tempdir().unwrap();
    let peer = Peer::new(&response(&[]), "delayed-ready");
    let output = peer.list(directory.path(), &["-r", "--json"]);
    success(&output);
    assert_eq!(output.stdout, b"{\"entries\":[],\"errors\":[]}\n");
}

#[test]
fn explicit_instance_queries_only_indexed_candidates_with_a_path_scoped_filename_filter() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfroot indexed",
    )
    .unwrap();
    fs::create_dir(directory.path().join("unindexed")).unwrap();
    fs::write(
        directory.path().join("unindexed/descript.ion"),
        b"\xef\xbb\xbfmissing omitted",
    )
    .unwrap();
    let peer = Peer::new(&response(&[directory.path().join("descript.ion")]), "reply");
    let output = peer.list(directory.path(), &["-r", "--json"]);
    success(&output);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"entries":[{"name":"root","comment":"indexed","extension":"none"}],"errors":[]})
    );
    let query = fs::read_to_string(peer.dir.path().join("query.txt")).unwrap();
    assert_eq!(
        query,
        format!("\"{}\\\" wfn:descript.ion", directory.path().display())
    );
}

#[test]
fn failed_ancestor_metadata_skips_its_subtree_and_preserves_other_indexed_records() {
    let directory = tempfile::tempdir().unwrap();
    let gate = directory.path().join("gate");
    let blocked = gate.join("blocked");
    fs::create_dir_all(blocked.join("deep")).unwrap();
    fs::create_dir(blocked.join("second")).unwrap();
    fs::create_dir(directory.path().join("later")).unwrap();
    fs::write(
        blocked.join("deep/descript.ion"),
        b"\xef\xbb\xbfhidden skipped",
    )
    .unwrap();
    fs::write(
        blocked.join("second/descript.ion"),
        b"\xef\xbb\xbfalso skipped",
    )
    .unwrap();
    fs::write(
        directory.path().join("later/descript.ion"),
        b"\xef\xbb\xbfafter kept",
    )
    .unwrap();
    let peer = Peer::new(
        &response(&[
            blocked.join("deep/descript.ion"),
            directory.path().join("later/descript.ion"),
            blocked.join("second/descript.ion"),
        ]),
        "reply",
    );
    let script = "$ErrorActionPreference='Stop'; $p=$env:DION_TEST_DIRECTORY; $acl=Get-Acl -LiteralPath $p; $rule=[Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.WindowsIdentity]::GetCurrent().User, 'ReadAttributes', 'Deny'); $acl.AddAccessRule($rule); Set-Acl -LiteralPath $p -AclObject $acl";
    let change_acl = |script: &str, path: &Path| {
        Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env_remove("PSModulePath")
            .env("DION_TEST_DIRECTORY", path)
            .output()
            .unwrap()
    };
    // std can fall back to enumerating the parent when the direct attributes query fails.
    let gate_script = script.replace("ReadAttributes", "ListDirectory");
    success(&change_acl(script, &blocked));
    success(&change_acl(&gate_script, &gate));
    let metadata_failed = fs::symlink_metadata(&blocked).is_err();
    let output = peer.list(directory.path(), &["-r", "--json"]);
    success(&change_acl(
        &gate_script.replace("AddAccessRule", "RemoveAccessRule"),
        &gate,
    ));
    success(&change_acl(
        &script.replace("AddAccessRule", "RemoveAccessRule"),
        &blocked,
    ));
    assert!(
        metadata_failed,
        "Fixture must deny the metadata lookup while allowing child access"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["entries"],
        serde_json::json!([
            {"name":"later\\after","comment":"kept","extension":"none"},
        ])
    );
    assert_eq!(value["errors"].as_array().unwrap().len(), 1);
    assert_eq!(value["errors"][0]["code"], "io_error");
    assert_eq!(value["errors"][0]["file"], blocked.to_str().unwrap());
}

#[test]
fn malformed_reply_is_a_localized_protocol_error_instead_of_a_timeout() {
    let directory = tempfile::tempdir().unwrap();
    let mut invalid_count = vec![0; 28];
    invalid_count[20..24].copy_from_slice(&1_u32.to_le_bytes());
    let mut overlapping_strings = vec![0; 40];
    overlapping_strings[20..24].copy_from_slice(&1_u32.to_le_bytes());
    overlapping_strings[32..36].copy_from_slice(&28_u32.to_le_bytes());
    overlapping_strings[36..40].copy_from_slice(&28_u32.to_le_bytes());
    let valid = response(&[directory.path().join("descript.ion")]);
    let mut unterminated = valid.clone();
    unterminated.truncate(unterminated.len() - 2);
    let mut invalid_utf16 = valid;
    let name_offset = u32::from_le_bytes(invalid_utf16[32..36].try_into().unwrap()) as usize;
    invalid_utf16[name_offset..name_offset + 2].copy_from_slice(&0xD800_u16.to_le_bytes());
    for bytes in [
        vec![],
        vec![0; 27],
        invalid_count,
        overlapping_strings,
        unterminated,
        invalid_utf16,
    ] {
        let peer = Peer::new(&bytes, "reply");
        let output = Command::new(env!("CARGO_BIN_EXE_dion"))
            .arg("list")
            .arg(directory.path())
            .args([
                "-r",
                "--everything",
                "--everything-instance",
                &peer.instance,
                "--json",
                "--lang",
                "zh-CN",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "everything_error");
        assert_eq!(error["error"]["message"], "Everything 返回了无效的结果列表");
    }
}

#[test]
fn indexed_results_match_scanning_in_all_layouts_with_exclusions_and_record_errors() {
    let directory = tempfile::tempdir().unwrap();
    let mut descriptions = vec![directory.path().join("descript.ion")];
    fs::write(
        &descriptions[0],
        b"\xef\xbb\xbfDIR2 directory note\n.git own note\nroot parent\n",
    )
    .unwrap();
    for (name, bytes) in [
        ("dir10", &b"\xef\xbb\xbften last\n"[..]),
        (
            "dir2",
            &b"\xef\xbb\xbftwo first\nTWO duplicate\n\"unclosed\n"[..],
        ),
        (
            "support/deep",
            &b"\xef\xbb\xbfchild nested\\nsecond\x04\xc3\x82\n"[..],
        ),
        (".git", &b"\xef\xbb\xbfexcluded restored\n"[..]),
        ("bad/deep", &b"\xef\xbb\xbfafter visited\n"[..]),
        ("bad", &b"missing BOM"[..]),
    ] {
        let dir = directory.path().join(name);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("descript.ion");
        fs::write(&file, bytes).unwrap();
        descriptions.push(file);
    }
    descriptions.reverse();
    let peer = Peer::new(&response(&descriptions), "reply");
    for options in [
        vec!["-r"],
        vec!["-r", "-l"],
        vec!["--tree"],
        vec!["-r", "--json"],
        vec!["--tree", "--json"],
        vec!["-r", "--all"],
        vec!["--tree", "--all"],
        vec!["-r", "--all", "--json"],
    ] {
        let scanned = cli(directory.path()).args(&options).output().unwrap();
        let indexed = peer.list(directory.path(), &options);
        assert_eq!(scanned.status.code(), Some(1));
        assert_eq!(indexed.status.code(), scanned.status.code());
        assert_eq!(indexed.stdout, scanned.stdout, "{options:?}");
        assert_eq!(indexed.stderr, scanned.stderr, "{options:?}");
    }
}

#[test]
fn indexed_zero_results_do_not_read_an_unindexed_bad_description() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("descript.ion"), b"missing BOM").unwrap();
    let peer = Peer::new(&response(&[]), "reply");
    let output = peer.list(directory.path(), &["--tree", "--json"]);
    success(&output);
    assert_eq!(output.stdout, b"{\"entries\":[],\"errors\":[]}\n");
}

#[test]
fn stale_missing_directory_candidates_are_skipped_without_hiding_valid_records() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfroot kept",
    )
    .unwrap();
    let peer = Peer::new(
        &response(&[
            directory.path().join("missing/deep/descript.ion"),
            directory.path().join("descript.ion"),
        ]),
        "reply",
    );
    let output = peer.list(directory.path(), &["-r", "--json"]);
    success(&output);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({"entries":[{"name":"root","comment":"kept","extension":"none"}],"errors":[]})
    );
}

#[test]
fn indexed_junctions_are_skipped_but_an_explicit_junction_root_is_read() {
    let directory = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    fs::create_dir(target.path().join("deep")).unwrap();
    fs::write(
        target.path().join("descript.ion"),
        b"\xef\xbb\xbfinside root\n",
    )
    .unwrap();
    fs::write(
        target.path().join("deep/descript.ion"),
        b"\xef\xbb\xbfchild nested\n",
    )
    .unwrap();
    let link = directory.path().join("link");
    let created = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "New-Item -ItemType Junction -Path $env:DION_TEST_LINK -Target $env:DION_TEST_TARGET | Out-Null"])
        .env_remove("PSModulePath").env("DION_TEST_LINK", &link).env("DION_TEST_TARGET", target.path())
        .output().unwrap();
    success(&created);
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfLINK own note\n",
    )
    .unwrap();
    let peer = Peer::new(
        &response(&[
            link.join("deep/descript.ion"),
            directory.path().join("descript.ion"),
            link.join("descript.ion"),
        ]),
        "reply",
    );
    let root = peer.list(directory.path(), &["--tree"]);
    let explicit = peer.list(&link, &["--tree", "--json"]);
    fs::remove_dir(&link).unwrap();
    success(&root);
    success(&explicit);
    assert_eq!(root.stdout, "└── LINK\\  own note\n".as_bytes());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&explicit.stdout).unwrap(),
        serde_json::json!({"entries":[
            {"name":"inside","comment":"root","extension":"none"},
            {"name":"deep\\child","comment":"nested","extension":"none"},
        ],"errors":[]})
    );
}

#[test]
fn ipc_readiness_version_rejection_and_timeout_are_fatal_without_scan_fallback() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfscan must not run",
    )
    .unwrap();
    for (mode, message) in [
        ("unready", "Everything database is not ready"),
        ("old", "Everything 1.4.1 or later is required"),
        ("reject", "Everything rejected the IPC query"),
        ("timeout", "Everything IPC timed out"),
    ] {
        let peer = Peer::new(&response(&[]), mode);
        let started = Instant::now();
        let output = peer.list(directory.path(), &["-r", "--json"]);
        assert_eq!(output.status.code(), Some(1), "{mode}");
        assert!(output.stdout.is_empty(), "{mode}");
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "everything_error");
        assert_eq!(error["error"]["message"], message);
        if mode == "timeout" {
            assert!(started.elapsed() >= Duration::from_secs(4));
        }
        if matches!(mode, "unready" | "old") {
            assert!(!peer.dir.path().join("query.txt").exists());
        }
    }
}

#[test]
fn default_instance_prefers_15a_falls_back_only_when_absent_and_never_after_failure() {
    static DEFAULT_INSTANCES: Mutex<()> = Mutex::new(());
    let _guard = DEFAULT_INSTANCES.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("descript.ion"),
        b"\xef\xbb\xbfroot preferred",
    )
    .unwrap();
    let Some(unnamed) = Peer::start("", &response(&[]), "reply") else {
        eprintln!(
            "UNVERIFIED: unnamed Everything instance already running; explicit-instance tests still apply"
        );
        return;
    };
    let Some(preferred) = Peer::start(
        "1.5a",
        &response(&[directory.path().join("descript.ion")]),
        "reply",
    ) else {
        eprintln!(
            "UNVERIFIED: Everything 1.5a already running; explicit-instance tests still apply"
        );
        return;
    };
    let run = || {
        cli(directory.path())
            .args(["-r", "--everything", "--json"])
            .output()
            .unwrap()
    };
    let output = run();
    success(&output);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["entries"][0]["name"],
        "root"
    );
    assert!(!unnamed.dir.path().join("query.txt").exists());
    drop(preferred);
    let unready = Peer::start("1.5a", &response(&[]), "unready").unwrap();
    let output = run();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!unnamed.dir.path().join("query.txt").exists());
    drop(unready);
    let output = run();
    success(&output);
    assert_eq!(output.stdout, b"{\"entries\":[],\"errors\":[]}\n");
    assert!(unnamed.dir.path().join("query.txt").exists());
}
