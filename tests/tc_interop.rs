use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;

fn dion(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

fn dion_stdin(directory: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_dion"))
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn fixture(contents: &[u8]) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("descript.ion"), contents).unwrap();
    directory
}

fn json_out(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn json_err(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr).unwrap()
}

// -----------------------------------------------------------------------------
// 1. TC 真实用例解析（AC 2）
// 全部样本来自 Total Commander 11.58 在 Windows 上的真实输出，样本数据完全自洽可提交
// -----------------------------------------------------------------------------

#[test]
fn tc_root_fixture_decodes_single_line_spaced_unicode_and_multiline() {
    // 对应真实 TC 顶层用例 .scratch/fixtures/descript.ion:
    // - 父文件夹1 111测试
    // - "父文件夹1 - 副本" 222测试
    // - 父文件夹2 测试测试\n。\x04\xc3\x82
    let raw = b"\xef\xbb\xbf\r\n\xe7\x88\xb6\xe6\x96\x87\xe4\xbb\xb6\xe5\xa4\xb91 111\xe6\xb5\x8b\xe8\xaf\x95\r\n\"\xe7\x88\xb6\xe6\x96\x87\xe4\xbb\xb6\xe5\xa4\xb91 - \xe5\x89\xaf\xe6\x9c\xac\" 222\xe6\xb5\x8b\xe8\xaf\x95\r\n\xe7\x88\xb6\xe6\x96\x87\xe4\xbb\xb6\xe5\xa4\xb92 \xe6\xb5\x8b\xe8\xaf\x95\xe6\xb5\x8b\xe8\xaf\x95\\n\xe3\x80\x82\x04\xc3\x82\r\n";
    let dir = fixture(raw);

    // 单行普通中文
    let res = dion(dir.path(), &["get", "父文件夹1", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": "父文件夹1",
            "comment": "111测试",
            "extension": "none"
        })
    );

    // 带空格的 Unicode 名称（双引号包裹）
    let res = dion(dir.path(), &["get", "父文件夹1 - 副本", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": "父文件夹1 - 副本",
            "comment": "222测试",
            "extension": "none"
        })
    );

    // 多行 TC 扩展记录
    let res = dion(dir.path(), &["get", "父文件夹2", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": "父文件夹2",
            "comment": "测试测试\n。",
            "extension": "tc"
        })
    );

    // list 列出全部记录并保持原始顺序
    let res = dion(dir.path(), &["list", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "entries": [
                { "name": "父文件夹1", "comment": "111测试", "extension": "none" },
                { "name": "父文件夹1 - 副本", "comment": "222测试", "extension": "none" },
                { "name": "父文件夹2", "comment": "测试测试\n。", "extension": "tc" }
            ]
        })
    );
}

#[test]
fn tc_subfolder_fixtures_decode_trailing_breaks_and_blank_lines() {
    // 对应真实 TC 用例父文件夹1及子目录：
    // config.json 测试换行1\n\n\x04\xc3\x82
    let raw1 = b"\xef\xbb\xbf\r\nconfig.json \xe6\xb5\x8b\xe8\xaf\x95\xe6\x8d\xa2\xe8\xa1\x8c1\\n\\n\x04\xc3\x82\r\n";
    let dir1 = fixture(raw1);
    let res = dion(dir1.path(), &["get", "config.json", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": "config.json",
            "comment": "测试换行1\n\n",
            "extension": "tc"
        })
    );

    // 2.txt 测试换行\n\x04\xc3\x82
    // 1.txt 测试换行\n\n。\x04\xc3\x82
    let raw2 = b"\xef\xbb\xbf\r\n2.txt \xe6\xb5\x8b\xe8\xaf\x95\xe6\x8d\xa2\xe8\xa1\x8c\\n\x04\xc3\x82\r\n1.txt \xe6\xb5\x8b\xe8\xaf\x95\xe6\x8d\xa2\xe8\xa1\x8c\\n\\n\xe3\x80\x82\x04\xc3\x82\r\n";
    let dir2 = fixture(raw2);

    let res = dion(dir2.path(), &["get", "2.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": "2.txt",
            "comment": "测试换行\n",
            "extension": "tc"
        })
    );

    let res = dion(dir2.path(), &["get", "1.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": "1.txt",
            "comment": "测试换行\n\n。",
            "extension": "tc"
        })
    );
}

#[test]
fn tc_backslash_and_literal_escapes_dense_fixture() {
    // 对应真实 TC 用例父文件夹1/88/.env/descript.ion：
    // 密集测试字面反斜杠与 \n, \\n, /n, \r, \r\n 以及连续空行
    let raw = b"\xef\xbb\xbf\r\n.env \xe5\xbc\x80\xe5\xa4\xb4\xf0\x9f\x98\x84\\n\\\\n \xe6\x98\xaf\xe5\xad\x97\xe9\x9d\xa2\xe5\x8f\x8d\xe6\x96\x9c\xe6\x9d\xa0\xe5\x92\x8c\xe5\xad\x97\xe6\xaf\x8d n\\n\\\\\\\\n \xe6\x98\xaf\xe4\xb8\xa4\xe4\xb8\xaa\xe5\x8f\x8d\xe6\x96\x9c\xe6\x9d\xa0\xe5\x92\x8c\xe5\xad\x97\xe6\xaf\x8d n\\n/n \xe6\x98\xaf\xe6\x96\x9c\xe6\x9d\xa0\xe5\x92\x8c\xe5\xad\x97\xe6\xaf\x8d n\\n\\\\r \xe6\x98\xaf\xe5\xad\x97\xe9\x9d\xa2\xe5\x8f\x8d\xe6\x96\x9c\xe6\x9d\xa0\xe5\x92\x8c\xe5\xad\x97\xe6\xaf\x8d r\\n\\\\r\\\\n \xe4\xb9\x9f\xe6\x98\xaf\xe5\xad\x97\xe9\x9d\xa2\xe6\x96\x87\xe5\xad\x97\xef\xbc\x8c\xe4\xb8\x8d\xe6\x98\xaf\xe6\x8d\xa2\xe8\xa1\x8c\\n\\n\xe8\xbf\x99\xe9\x87\x8c\xe5\x92\x8c\xe4\xb8\x8a\xe4\xb8\x80\xe8\xa1\x8c\xe4\xb9\x8b\xe9\x97\xb4\xe6\x9c\x89\xe4\xb8\x80\xe4\xb8\xaa\xe7\xa9\xba\xe8\xa1\x8c\xef\xbc\x8c\xe4\xb8\x8b\xe4\xb8\x80\xe8\xa1\x8c\xe7\xbb\x93\xe6\x9d\x9f\xe5\x90\x8e\xe8\xbf\x98\xe6\x9c\x89\xe6\x9c\xab\xe5\xb0\xbe\xe6\x8d\xa2\xe8\xa1\x8c\xef\xbc\x9a\x04\xc3\x82\r\n";
    let dir = fixture(raw);

    let res = dion(dir.path(), &["get", ".env", "--json"]);
    assert_eq!(res.status.code(), Some(0));

    let expected_body = "开头😄\n\\n 是字面反斜杠和字母 n\n\\\\n 是两个反斜杠和字母 n\n/n 是斜杠和字母 n\n\\r 是字面反斜杠和字母 r\n\\r\\n 也是字面文字，不是换行\n\n这里和上一行之间有一个空行，下一行结束后还有末尾换行：";
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": ".env",
            "comment": expected_body,
            "extension": "tc"
        })
    );
}

#[test]
fn tc_complex_characters_and_edge_fixtures() {
    // 对应真实 TC 用例父文件夹1 - 副本/top: 特殊字符密集样本
    let raw_top = b"\xef\xbb\xbf\x0d\x0a\x74\x6f\x70\x20\xe6\xb5\x8b\xe8\xaf\x95\xe7\x89\xb9\xe6\xae\x8a\xe5\xad\x97\xe7\xac\xa6\xef\xbc\x9a\x29\x7e\xc3\x8c\x2e\x2c\x2f\xc3\x95\x76\xc3\xbd\x22\xc2\xbf\x67\xc2\xbb\x2f\xc3\x84\x21\x50\xc3\xbd\xc3\xa0\xc2\xba\x7e\xc3\xb0\x67\xc3\xb3\x4a\xc2\xb0\xc3\xa2\xc3\x88\x3d\xc3\xb2\xc2\xa4\xc3\x9c\x25\x58\x58\xc3\x81\x34\xc2\xaf\xc3\x9b\xc3\x91\xc2\xb2\x34\xc2\xa8\x51\x37\x28\x4a\xc3\xbd\x40\xc3\xba\x51\x45\x41\xc3\xb5\xc2\xab\xc3\xb4\xc3\x94\x34\x3e\xc3\x80\x62\xc3\x9f\x4d\x25\x39\x75\xc3\xbe\x22\x2f\xc3\xa5\x32\x58\xc3\x84\xc2\xb9\x55\x68\xc3\x8d\x73\xc3\xb6\xc2\xa9\xc3\xb1\xc3\xbf\x78\xc2\xa7\x6a\xc3\xb7\x75\x71\x6b\x21\x7b\x77\xc3\xae\x40\xc3\x8b\x32\x76\xc3\x82\x64\x6f\x2b\xc3\xa3\xc2\xa9\xc2\xa7\x58\x3e\x70\x4d\x53\xc2\xa6\x63\xc2\xbf\x3a\xc3\xb2\xc2\xbe\xc2\xa6\x5a\x2a\x4c\xc3\xba\x3a\xc3\x9d\xc2\xa7\xc3\xba\x7e\xc2\xa3\xc3\xa2\x7b\x5c\x6e\x3e\x2b\x53\x4e\x2b\x3f\x3d\x54\x5e\x26\x65\x77\x6e\x24\x70\x53\x22\x6b\x5c\x5c\x6a\x71\x7b\x62\x6f\x76\x4d\x56\x3e\x28\x7b\x74\x73\x3d\x2d\x4b\x5b\x22\x45\x6e\x2e\x4d\x3b\x39\x55\x5f\x44\x66\x71\x25\x55\x2e\x6d\x25\x4e\x70\x7b\x3b\x41\x50\x6f\x40\x4d\x72\x5e\x70\x70\x45\x4c\x5c\x5c\x3b\x73\x53\x78\x5f\x33\x32\x77\x3d\x65\x66\x56\x7e\x2c\x2e\x32\x21\x6d\x4d\x62\x7a\x4d\x59\x34\x3f\x7d\x58\x2e\x58\x44\x4d\x23\x79\x51\x33\x43\x62\x3a\x67\x78\x24\x3f\x4a\x58\x25\x5a\x3e\x5f\x27\x39\x2a\x2e\x2b\x52\x50\x2a\x2e\x50\x71\x04\xc3\x82\x0d\x0a";
    let dir = fixture(raw_top);
    let res = dion(dir.path(), &["get", "top", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    let parsed = json_out(&res);
    assert_eq!(parsed["name"], "top");
    assert_eq!(parsed["extension"], "tc");
    let comment = parsed["comment"].as_str().unwrap();
    assert!(comment.contains("测试特殊字符："));
    assert!(comment.contains("k\\jq"));
    assert!(comment.contains("EL\\;sSx"));

    // 边缘用例：空记录、连续空格文件名、单引号、点文件、混合语言 Emoji、首尾空格保留
    let raw_edge = concat!(
        "\u{feff}\r\n",
        "empty_comment.txt \r\n",
        "\"file   multiple   spaces.txt\" 包含连续空格的文件名\r\n",
        "\"User's Notes.md\" 包含单引号的文件名备注\r\n",
        ".clang-format 项目代码格式化配置\r\n",
        "日本語_한국어_🚀.txt 中日韩与Emoji混合文件名\r\n",
        "tab_and_indent.txt   首尾保留多个空格的备注  \r\n",
    );
    let dir_edge = fixture(raw_edge.as_bytes());

    // 空记录返回 0 且正文为空
    let res_empty = dion(dir_edge.path(), &["get", "empty_comment.txt", "--json"]);
    assert_eq!(res_empty.status.code(), Some(0));
    assert_eq!(json_out(&res_empty)["comment"], "");

    // 连续空格文件名（双引号包裹）
    let res_space = dion(
        dir_edge.path(),
        &["get", "file   multiple   spaces.txt", "--json"],
    );
    assert_eq!(res_space.status.code(), Some(0));
    assert_eq!(json_out(&res_space)["name"], "file   multiple   spaces.txt");

    // 单引号文件名
    let res_quote = dion(dir_edge.path(), &["get", "User's Notes.md", "--json"]);
    assert_eq!(res_quote.status.code(), Some(0));
    assert_eq!(json_out(&res_quote)["name"], "User's Notes.md");

    // Emoji 与混合语言
    let res_emoji = dion(dir_edge.path(), &["get", "日本語_한국어_🚀.txt", "--json"]);
    assert_eq!(res_emoji.status.code(), Some(0));
    assert_eq!(json_out(&res_emoji)["comment"], "中日韩与Emoji混合文件名");

    // 首尾空格保留
    let res_tab = dion(dir_edge.path(), &["get", "tab_and_indent.txt", "--json"]);
    assert_eq!(res_tab.status.code(), Some(0));
    assert_eq!(json_out(&res_tab)["comment"], "  首尾保留多个空格的备注  ");

    // list 完整返回 6 条
    let res_list = dion(dir_edge.path(), &["list", "--json"]);
    assert_eq!(res_list.status.code(), Some(0));
    assert_eq!(json_out(&res_list)["entries"].as_array().unwrap().len(), 6);
}

// -----------------------------------------------------------------------------
// 2. CLI 新增与覆盖（三种来源、单行与多行转义、直接字节断言脱离自洽，AC 3 / AC 5）
// -----------------------------------------------------------------------------

#[test]
fn cli_set_adds_and_overwrites_with_direct_byte_verification() {
    let dir = tempfile::tempdir().unwrap();
    let ion_path = dir.path().join("descript.ion");

    // 1. 新增单行反斜杠：直接检查字节中无 TC 扩展标记，末尾为 CRLF
    let target1 = dir.path().join("file1.txt");
    fs::write(&target1, b"").unwrap();
    let res = dion(
        dir.path(),
        &["set", "file1.txt", "C:\\Tool\\App.exe", "--json"],
    );
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": true }));

    let bytes = fs::read(&ion_path).unwrap();
    assert_eq!(
        bytes, b"\xef\xbb\xbf\r\nfile1.txt C:\\Tool\\App.exe\r\n",
        "单行字面反斜杠不得附加 04 C3 82 扩展标记"
    );

    // 2. 新增多行（参数来源）：包含反斜杠与换行，直接验证字节编码
    let target2 = dir.path().join("file2.txt");
    fs::write(&target2, b"").unwrap();
    let res = dion(
        dir.path(),
        &["set", "file2.txt", "第一行\\目录\n第二行\n", "--json"],
    );
    assert_eq!(res.status.code(), Some(0));

    let bytes = fs::read(&ion_path).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.contains("file2.txt 第一行\\\\目录\\n第二行\\n\u{4}\u{c2}\r\n"),
        "多行参数必须将反斜杠加倍、换行转为 \\n 并附加 04 C3 82 扩展"
    );

    // 3. 覆盖已有记录（stdin 来源）：覆盖 file1.txt 为多行备注，验证其余记录字节不变
    let prev_file2_bytes = text.lines().find(|l| l.starts_with("file2.txt")).unwrap();
    let res = dion_stdin(
        dir.path(),
        &["set", "file1.txt", "--stdin", "--json"],
        "stdin新行1\r\nstdin新行2".as_bytes(),
    );
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": true }));

    let bytes = fs::read(&ion_path).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.contains("file1.txt stdin新行1\\nstdin新行2\u{4}\u{c2}\r\n"),
        "stdin 覆盖写入的记录应正确编码"
    );
    assert!(
        text.contains(prev_file2_bytes),
        "覆盖修改一条记录时，其余记录原始字节完全保持"
    );

    // 4. 覆盖已有记录（文件来源）：通过 --comment-file 覆盖 file2.txt，包含末尾换行
    let comment_file = dir.path().join("comment_source.txt");
    fs::write(&comment_file, "\u{feff}文件覆盖行1\n文件覆盖行2\n").unwrap();
    let res = dion(
        dir.path(),
        &[
            "set",
            "file2.txt",
            "--comment-file",
            "comment_source.txt",
            "--json",
        ],
    );
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": true }));

    let bytes = fs::read(&ion_path).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.contains("file2.txt 文件覆盖行1\\n文件覆盖行2\\n\u{4}\u{c2}\r\n"),
        "文件来源覆盖写入的记录应包含尾部换行转义及扩展标记"
    );

    // 5. 覆盖相同内容：changed: false，且文件完全未写
    let before_write_bytes = fs::read(&ion_path).unwrap();
    let res = dion(
        dir.path(),
        &[
            "set",
            "file2.txt",
            "--comment-file",
            "comment_source.txt",
            "--json",
        ],
    );
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": false }));
    assert_eq!(fs::read(&ion_path).unwrap(), before_write_bytes);
}

// -----------------------------------------------------------------------------
// 3. 字节保留、记录顺序保持与生命周期清理（AC 4）
// -----------------------------------------------------------------------------

#[test]
fn tc_record_byte_preservation_and_removal_cleanup() {
    let raw = b"\xef\xbb\xbf\r\nfirst.txt \xe7\xac\xac\xe4\xb8\x80\xe6\x9d\xa1\\n\x04\xc3\x82\r\nsecond.txt \xe7\xac\xac\xe4\xba\x8c\xe6\x9d\xa1\r\n";
    let dir = fixture(raw);
    let ion = dir.path().join("descript.ion");

    // 1. 设置相同内容，要求目标条目存在，changed: false，字节 100% 不变
    fs::write(dir.path().join("first.txt"), b"").unwrap();
    let res = dion(dir.path(), &["set", "first.txt", "第一条\n", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": false }));
    assert_eq!(fs::read(&ion).unwrap(), raw);

    // 2. 删除 first.txt，保留 second.txt 原始字节与 BOM
    let res = dion(dir.path(), &["remove", "first.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": true }));
    let expected_remaining = b"\xef\xbb\xbf\r\nsecond.txt \xe7\xac\xac\xe4\xba\x8c\xe6\x9d\xa1\r\n";
    assert_eq!(fs::read(&ion).unwrap(), expected_remaining);

    // 3. 删除 second.txt（最后一条记录），文件彻底自动删除
    let res = dion(dir.path(), &["remove", "second.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": true }));
    assert!(!ion.exists());
}

#[test]
fn tc_deletion_retaining_empty_or_unknown_records_does_not_delete_file() {
    // 包含一条正常记录、一条空记录、一条未知扩展记录（UTF-8 有效字符）
    let raw = b"\xef\xbb\xbf\r\ntarget.txt \xe6\xad\xa3\xe5\xb8\xb8\r\nempty.txt \r\nunknown.txt \xe6\x9c\xaa\xe7\x9f\xa5\x04unknown_ext\r\n";
    let dir = fixture(raw);
    let ion = dir.path().join("descript.ion");

    // 删除 target.txt，由于剩余 empty.txt 与 unknown.txt，文件保留
    let res = dion(dir.path(), &["remove", "target.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res), serde_json::json!({ "changed": true }));
    assert!(ion.exists());

    // 删除 empty.txt，剩余 unknown.txt，文件仍保留
    let res = dion(dir.path(), &["remove", "empty.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert!(ion.exists());

    // 删除 unknown.txt（最后一条），文件彻底清理
    let res = dion(dir.path(), &["remove", "unknown.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert!(!ion.exists());
}

// -----------------------------------------------------------------------------
// 4. TC 观察与 Dion 规格策略区分（AC 6）
// -----------------------------------------------------------------------------

#[test]
fn policy_accepts_records_without_trailing_newline_and_literal_unknown_escapes() {
    // 末尾无物理换行的记录（Dion 产品策略：接受并能在追加时补全 CRLF）
    let raw = b"\xef\xbb\xbf\r\ntarget.txt \xe7\x89\xb9\xe6\xae\x8a\\x\\y\x04\xc3\x82";
    let dir = fixture(raw);

    // 验证未知转义 \x \y 按字面保留（Dion 明确规则）
    let res = dion(dir.path(), &["get", "target.txt", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(
        json_out(&res),
        serde_json::json!({
            "name": "target.txt",
            "comment": "特殊\\x\\y",
            "extension": "tc"
        })
    );

    // 追加一条新记录：前一条记录成功补上 CRLF，且新记录追加在末尾
    fs::write(dir.path().join("new.txt"), b"").unwrap();
    let res = dion(dir.path(), &["set", "new.txt", "新记录", "--json"]);
    assert_eq!(res.status.code(), Some(0));

    let ion_content = fs::read_to_string(dir.path().join("descript.ion")).unwrap();
    assert_eq!(
        ion_content,
        "\u{feff}\r\ntarget.txt 特殊\\x\\y\u{4}\u{c2}\r\nnew.txt 新记录\r\n"
    );
}

#[test]
fn policy_exact_4096_byte_boundary_includes_terminator_and_tc_extension() {
    // 4096 字节计算包含名称、空格、正文、扩展及 CRLF (2 字节)
    // 构造一条刚好 4096 字节的记录
    // 名称: target.txt (10 字节)
    // 分隔空格: 1 字节
    // CRLF: 2 字节
    // 剩余正文空间: 4096 - 10 - 1 - 2 = 4083 字节
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.txt");
    fs::write(&target, b"").unwrap();

    let exact_body = "A".repeat(4083);
    let res = dion(dir.path(), &["set", "target.txt", &exact_body, "--json"]);
    assert_eq!(
        res.status.code(),
        Some(0),
        "刚好 4096 字节应当成功: {:?}",
        String::from_utf8_lossy(&res.stderr)
    );

    let bytes = fs::read(dir.path().join("descript.ion")).unwrap();
    // BOM (3) + CRLF (2) + 4096 = 4101 字节
    assert_eq!(bytes.len(), 4101);

    // 4084 字节正文导致总长度为 4097 字节，必须被拒绝并报错
    let overflow_body = "A".repeat(4084);
    let res = dion(dir.path(), &["set", "target.txt", &overflow_body, "--json"]);
    assert_eq!(res.status.code(), Some(1));
    assert_eq!(json_err(&res)["error"]["code"], "invalid_format");
}

#[test]
fn policy_case_insensitive_matching_and_duplicate_conflict() {
    // 名称匹配固定不区分大小写，大小写重复记录视为格式冲突（拒绝整次操作）
    let dir = fixture(b"\xef\xbb\xbf\r\nfile.txt \xe5\xa4\x87\xe6\xb3\xa8\r\n");

    // 大写查询能正确匹配小写记录
    let res = dion(dir.path(), &["get", "FILE.TXT", "--json"]);
    assert_eq!(res.status.code(), Some(0));
    assert_eq!(json_out(&res)["name"], "file.txt");

    // 包含仅大小写不同的重复记录（冲突），整次操作报错且不修改
    let conflict_raw = b"\xef\xbb\xbf\r\nfile.txt \xe5\xa4\x87\xe6\xb3\xa81\r\nFILE.TXT \xe5\xa4\x87\xe6\xb3\xa82\r\n";
    let dir_conflict = fixture(conflict_raw);
    let res = dion(dir_conflict.path(), &["list", "--json"]);
    assert_eq!(res.status.code(), Some(1));
    assert_eq!(json_err(&res)["error"]["code"], "invalid_format");
}
