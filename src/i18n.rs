use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    fn parse(value: &str) -> Option<Option<Self>> {
        if value.eq_ignore_ascii_case("auto") {
            Some(None)
        } else if value.eq_ignore_ascii_case("en") {
            Some(Some(Self::English))
        } else if value.eq_ignore_ascii_case("zh-CN") {
            Some(Some(Self::Chinese))
        } else {
            None
        }
    }

    pub fn from_environment() -> Self {
        env::var("DION_LANG")
            .ok()
            .filter(|value| !value.is_empty())
            .and_then(|value| Self::parse(&value).flatten())
            .unwrap_or_else(Self::from_user_interface)
    }

    #[cfg(windows)]
    fn from_user_interface() -> Self {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }

        // SAFETY: This Windows API has no pointer or handle arguments.
        let language = unsafe { GetUserDefaultUILanguage() };
        Self::from_langid(language)
    }

    #[cfg(not(windows))]
    fn from_user_interface() -> Self {
        Self::English
    }

    fn from_langid(language: u16) -> Self {
        match language {
            0x0004 | 0x0804 | 0x1004 | 0x7804 => Self::Chinese,
            _ => Self::English,
        }
    }

    pub fn resolve_option(value: &str) -> Option<Self> {
        Self::parse(value).map(|language| language.unwrap_or_else(Self::from_user_interface))
    }

    pub fn is_chinese(self) -> bool {
        self == Self::Chinese
    }
}

pub fn error_message(code: &str, message: &str, language: Language) -> String {
    if !language.is_chinese() {
        return english_dynamic_message(message);
    }
    if let Some((base, path)) = message.split_once("; Edit text retained at ") {
        return format!(
            "{}；编辑文本已保留在 {path}",
            error_message(code, base, language)
        );
    }
    if let Some(detail) = message.strip_prefix("Could not run editor: ") {
        return format!("无法启动编辑器：{detail}");
    }
    if let Some(detail) = message.strip_prefix("Editor exited unsuccessfully: ") {
        return format!("编辑器异常退出：{detail}");
    }
    if let Some(detail) = message.strip_prefix("Commit failed: ") {
        if let Some((system, recovery)) = detail.split_once(". Inspect description file ")
            && let Some((file, temporary)) = recovery.split_once(" and temporary file ")
            && let Some((temporary, _)) =
                temporary.split_once(" for recovery; replacement may be partially complete")
        {
            return format!(
                "提交备注失败：{system}。检查备注文件 {file} 和临时文件 {temporary} 以恢复；替换可能只完成了一部分"
            );
        }
        return format!("提交备注失败：{detail}");
    }
    match message {
        "--color requires auto, always or never" => "--color 的值必须为 auto、always 或 never".into(),
        "--lang requires en, zh-CN or auto" => "--lang 的值必须为 en、zh-CN 或 auto".into(),
        "--comment-file requires a file; prefix a filename starting with - with ./" => {
            "--comment-file 需要文件路径；以 - 开头的文件名请添加 ./ 前缀".into()
        }
        "Unknown or repeated option; use -- before a path starting with -" => {
            "未知或重复的选项；路径以 - 开头时请在前面使用 --".into()
        }
        "--tree and --long are mutually exclusive" => "--tree 和 --long 不能同时使用".into(),
        "Usage: dion help [command]" => "用法：dion help [command]".into(),
        "Unknown help command" => "未知的帮助命令".into(),
        "--long, --recursive, --tree and --color are only supported by list" => {
            "--long、--recursive、--tree 和 --color 仅适用于 list 命令".into()
        }
        "Usage: dion [--json|-j] get <path> | remove <path> | list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--color auto|always|never] | set <path> (<comment> | --stdin|-i | --comment-file|-f <file> | --edit|-e); use dion help for command aliases" => {
            "用法：dion [--json|-j] get <path> | remove <path> | list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--color auto|always|never] | set <path> (<comment> | --stdin|-i | --comment-file|-f <file> | --edit|-e)；使用 dion help 查看命令别名".into()
        }
        "Path must have a UTF-8 entry name" => "路径中的条目名称必须是 UTF-8".into(),
        "Path has no parent" => "路径没有上级目录".into(),
        "Comment must be UTF-8" => "备注必须是 UTF-8".into(),
        "Editing requires a target path" => "编辑备注需要指定目标路径".into(),
        "Comment input is not valid UTF-8" => "备注输入不是有效的 UTF-8".into(),
        "A nonblank comment without NUL or control character 04 is required" => {
            "备注不能为空或纯空白，也不能包含 NUL 或控制字符 04".into()
        }
        "Description file content changed while editing" => "编辑期间备注文件内容发生变化".into(),
        "Comment not found" => "未找到备注".into(),
        "Path must not be empty" => "路径不能为空".into(),
        "Invalid editor command line" => "编辑器命令行无效".into(),
        "Editing comments requires Windows" => "编辑备注需要 Windows".into(),
        "Deleting comments requires Windows" => "删除备注需要 Windows".into(),
        "Writing comments requires Windows" => "写入备注需要 Windows".into(),
        "Parent is not a directory" => "上级路径不是目录".into(),
        "Directory name is not valid UTF-8" => "目录名称不是有效的 UTF-8".into(),
        "Missing UTF-8 BOM" => "缺少 UTF-8 BOM".into(),
        "Record is not valid UTF-8" => "记录不是有效的 UTF-8".into(),
        "Physical record exceeds 4096 bytes" => "备注记录超过 4096 字节".into(),
        "Unclosed name quote" => "文件名引号未闭合".into(),
        "Expected a space after quoted name" => "带引号的文件名后需要空格".into(),
        "Record name is empty" => "记录名称为空".into(),
        "Duplicate record name (case insensitive)" => "记录名称重复（不区分大小写）".into(),
        "Unknown extension; cannot modify this record" => "程序扩展未知，无法修改此记录".into(),
        "Serialized record exceeds 4096 bytes" => "序列化后的备注记录超过 4096 字节".into(),
        "Final record exceeds 4096 bytes after adding CRLF" => "添加 CRLF 后，末条记录超过 4096 字节".into(),
        "Description file content changed before commit" => "提交前备注文件内容发生变化".into(),
        "Description file content changed before deletion" => "删除前备注文件内容发生变化".into(),
        "Description file is read-only" => "备注文件为只读".into(),
        _ if code == "io_error" => format!("文件操作失败：{message}"),
        _ => message.to_owned(),
    }
}

fn english_dynamic_message(message: &str) -> String {
    if let Some((base, path)) = message.split_once("; Edit text retained at ") {
        return format!(
            "{}; Edit text retained at {path}",
            english_dynamic_message(base)
        );
    }
    message.to_owned()
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn windows_ui_language_maps_only_simplified_chinese_to_chinese() {
        for language in [0x0004, 0x0804, 0x1004, 0x7804] {
            assert_eq!(Language::from_langid(language), Language::Chinese);
        }
        for language in [0x0404, 0x0c04, 0x1404, 0x7c04, 0x0409] {
            assert_eq!(Language::from_langid(language), Language::English);
        }
    }

    #[test]
    fn accepted_language_names_are_case_insensitive() {
        assert_eq!(Language::resolve_option("EN"), Some(Language::English));
        assert_eq!(Language::resolve_option("zh-cn"), Some(Language::Chinese));
        assert!(Language::resolve_option("fr").is_none());
    }
}
