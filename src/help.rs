use crate::i18n::Language;

pub const OVERVIEW: &str = r#"Dion — 文件与文件夹备注

用途：查看、列出、设置和删除 descript.ion 中的备注。

用法：
  dion get <path>
  dion list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--color auto|always|never]
  dion set <path> (<comment> | --stdin|-i | --comment-file|-f <file> | --edit|-e)
  dion remove <path>
  dion help [command]

命令：
  get (view, cat)          查看条目自身的备注
  list (ls)               列出目录内部条目的备注，默认当前目录、双栏展示
  set                     设置条目自身的备注
  remove (rm, unset, del)  删除条目自身的备注

括号内为命令别名；help <别名> 和 <别名> -h 显示对应命令帮助。

全局参数：
  --json, -j  输出 JSON；帮助仍为文本
  --help, -h  显示帮助
  --          结束选项解析，以 - 开头的路径或正文放在其后

选项分开输入；不支持短选项连写、紧连传值或等号传值。
"#;

const OVERVIEW_EN: &str = r#"Dion — file and folder comments

Purpose: View, list, set, and remove comments in descript.ion.

Usage:
  dion get <path>
  dion list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--color auto|always|never]
  dion set <path> (<comment> | --stdin|-i | --comment-file|-f <file> | --edit|-e)
  dion remove <path>
  dion help [command]

Commands:
  get (view, cat)          Show a file or folder comment
  list (ls)               List comments inside a directory; defaults to the current directory
  set                     Set a comment for a file or folder
  remove (rm, unset, del)  Remove a comment from a file or folder

Names in parentheses are aliases. Use help <alias> or <alias> -h for command help.

Global options:
  --json, -j              Output JSON; help remains plain text
  --help, -h              Show help
  --                      End option parsing; put paths or comments starting with - after it

Options take separate arguments. Combined short options, attached values, and = values are not supported.
"#;

/// Returns the Chinese help for an existing command without performing an operation.
pub fn command(name: &str) -> Option<&'static str> {
    match name {
        "get" => Some(GET),
        "list" => Some(LIST),
        "set" => Some(SET),
        "remove" => Some(REMOVE),
        _ => None,
    }
}

pub fn overview(language: Language) -> &'static str {
    if language.is_chinese() {
        OVERVIEW
    } else {
        OVERVIEW_EN
    }
}

pub fn command_for(name: &str, language: Language) -> Option<&'static str> {
    if language.is_chinese() {
        return command(name);
    }
    match name {
        "get" => Some(GET_EN),
        "list" => Some(LIST_EN),
        "set" => Some(SET_EN),
        "remove" => Some(REMOVE_EN),
        _ => None,
    }
}

const GET_EN: &str = r#"Purpose: Show the comment for a file or folder, stored in its parent directory's descript.ion.

Usage: dion get <path> [--json|-j]
Aliases: view, cat

Arguments and defaults:
  <path>      Entry path; the target does not need to exist
  --json, -j  Output JSON; by default output only the comment without an extra newline
  --help, -h  Show help

Exit code 3 means no comment was found. An existing empty comment succeeds.
"#;

const LIST_EN: &str = r#"Purpose: List comments for entries inside a directory, including orphan records.

Usage: dion list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--color auto|always|never] [--json|-j]
Alias: ls

Arguments and defaults:
  [directory]      Defaults to the current directory
  --long, -l       Use one-column output
  --recursive, -r  Include subdirectories; names are relative to the starting directory
  --tree, -t       Show a tree and recurse automatically; conflicts with --long
  --color <mode>   Name color: auto (default), always, or never
  --json, -j       Output JSON; tree mode returns ordinary recursive JSON
  --help, -h       Show help

By default, list the current directory only, in aligned columns with directories first and natural name sorting.
auto enables color only in a supported terminal. A nonempty NO_COLOR disables default color; always overrides it.
JSON output never includes color codes.
Recursive listing does not follow directory symlinks or junctions, unless one is the starting directory.
Bad records and read errors are skipped by default; text output summarizes errors and JSON returns entries and errors.
Skipped errors use exit code 1; an inaccessible starting directory fails immediately.
"#;

const SET_EN: &str = r#"Purpose: Set a comment for a file or folder in its parent directory's descript.ion.

Usage:
  dion set <path> <comment>
  dion set <path> --stdin|-i
  dion set <path> --comment-file|-f <file>
  dion set <path> --edit|-e

Arguments and defaults:
  <path>                    Entry path; a nonempty comment requires the target to exist
  <comment>                 Comment text; literal \n is not converted to a line break
  --stdin, -i               Read the comment from standard input
  --comment-file, -f <file> Read the comment from a UTF-8 file; an optional BOM is accepted
  --edit, -e                Edit the existing comment, or start with empty text
  --json, -j                Output a changed boolean; success is silent by default
  --help, -h                Show help

Comment sources are mutually exclusive. Text arguments, stdin, and files reject empty or whitespace-only content.
Stdin and editor text must be UTF-8; an optional BOM is accepted.
The editor is selected from VISUAL, EDITOR, then Windows Notepad, and Dion waits for it to exit.
Clearing the editor text removes the comment; whitespace-only text is still invalid. Unchanged text is not written.
If editing fails, the temporary text is retained and its recovery path is reported on stderr.
"#;

const REMOVE_EN: &str = r#"Purpose: Remove a file or folder comment while preserving the target entry.

Usage: dion remove <path> [--json|-j]
Aliases: rm, unset, del

Arguments and defaults:
  <path>      Entry path; the target does not need to exist
  --json, -j  Output a changed boolean; success is silent by default
  --help, -h  Show help

If no record exists, the command succeeds without changes. Removing the last record deletes descript.ion.
"#;

const GET: &str = r#"用途：查看文件或文件夹条目自身的备注，从父目录的 descript.ion 读取。

用法：dion get <path> [--json|-j]
别名：view、cat

参数与默认行为：
  <path>      条目路径，无需目标存在
  --json, -j  输出 JSON；默认只输出正文，不额外添加换行
  --help, -h  显示帮助

未找到备注时退出码为 3；已有空记录读取成功。
"#;

const LIST: &str = r#"用途：列出目录内部条目的备注，包括孤立记录。

用法：dion list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--color auto|always|never] [--json|-j]
别名：ls

参数与默认行为：
  [directory]      默认当前目录
  --long, -l       单栏展示
  --recursive, -r  递归查询子目录，名称相对于起始目录
  --tree, -t       树状展示，自动递归，与 --long 互斥
  --color <mode>   名称颜色：auto（默认）、always 或 never
  --json, -j       输出 JSON；树模式输出普通递归 JSON
  --help, -h       显示帮助

默认只查询当前层，双栏展示完整正文，目录优先、名称自然排序。
auto 仅在支持颜色的终端启用；非空 NO_COLOR 可关闭默认颜色，always 可覆盖。
JSON 输出始终不添加颜色。
递归不进入途中遇到的目录符号链接或 Junction；显式指定为起始目录时读取内部。
默认跳过坏记录和读取错误，继续查询；文本底部汇总错误，JSON 返回 entries 和 errors。
有跳过的错误时退出码为 1；起始目录无法访问时直接失败。
"#;

const SET: &str = r#"用途：设置文件或文件夹条目自身的备注，保存在父目录的 descript.ion。

用法：
  dion set <path> <comment>
  dion set <path> --stdin|-i
  dion set <path> --comment-file|-f <file>
  dion set <path> --edit|-e

参数与默认行为：
  <path>                    条目路径；设置非空正文要求目标存在
  <comment>                 正文参数，字面量 \n 不转换为换行
  --stdin, -i               从标准输入读取正文
  --comment-file, -f <file>  从 UTF-8 文件读取正文，接受可选 BOM
  --edit, -e                编辑已有正文，无记录时打开空文本
  --json, -j                输出 changed 布尔值；默认成功时静默
  --help, -h                显示帮助

四种正文来源互斥；正文参数、stdin 和正文文件拒绝空字符串及纯空白。
stdin 和编辑文本也须为 UTF-8，接受可选 BOM。
编辑器依次选择非空 VISUAL、EDITOR、Windows 记事本，并等待正常退出；
需要等待参数的编辑器须自行配置。
--edit 清空删除备注，纯空白仍报错；有效正文未变化时不写入。
编辑失败保留已创建的编辑文本，并在 stderr 报告恢复路径。
"#;

const REMOVE: &str = r#"用途：删除文件或文件夹条目自身的备注，保留目标条目。

用法：dion remove <path> [--json|-j]
别名：rm、unset、del

参数与默认行为：
  <path>      条目路径，无需目标存在
  --json, -j  输出 changed 布尔值；默认成功时静默
  --help, -h  显示帮助

无记录时成功且不修改文件；删除最后一条记录时删除 descript.ion。
"#;
