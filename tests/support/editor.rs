use std::{
    env, fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    assert_eq!(args.len(), 3);
    assert_eq!(args[0], "--label");
    assert_eq!(
        args[1],
        env::var("EDIT_EXPECT_LABEL")
            .unwrap_or_else(|_| "quoted value".into())
            .as_str()
    );
    let text = PathBuf::from(args.last().unwrap());
    fs::copy(&text, "prefill.txt").unwrap();
    fs::write("edit-path.txt", text.to_str().unwrap()).unwrap();
    fs::write("started", b"").unwrap();
    if env::var_os("EDIT_GATE").is_some() {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !PathBuf::from("continue").exists() {
            assert!(Instant::now() < deadline, "editor gate timed out");
            thread::sleep(Duration::from_millis(10));
        }
    }
    if let Some(body) = env::var_os("EDIT_BODY") {
        fs::copy(body, &text).unwrap();
    }
    match env::var("EDIT_CONFLICT").as_deref() {
        Ok("write") => {
            fs::copy("other.ion", "descript.ion").unwrap();
        }
        Ok("delete") => {
            fs::remove_file("descript.ion").unwrap();
        }
        _ => {}
    }
    std::process::exit(
        env::var("EDIT_EXIT")
            .unwrap_or_else(|_| "0".into())
            .parse()
            .unwrap(),
    );
}
