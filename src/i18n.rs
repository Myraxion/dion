use crate::error::Diagnostic;
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

impl Diagnostic {
    pub fn text(self, language: Language) -> &'static str {
        let (english, chinese) = match self {
            Self::EverythingInstanceNotFound => {
                ("Everything instance not found", "找不到 Everything 实例")
            }
            Self::EverythingDatabaseIsNotReady => (
                "Everything database is not ready",
                "Everything 数据库尚未就绪",
            ),
            Self::EverythingIpcTimedOut => ("Everything IPC timed out", "Everything IPC 查询超时"),
            Self::UnsupportedEverythingVersion => (
                "Everything 1.4.1 or later is required",
                "需要 Everything 1.4.1 或更高版本",
            ),
            Self::InvalidColor => (
                "--color requires auto, always or never",
                "--color 的值必须为 auto、always 或 never",
            ),
            Self::EverythingInstanceRequiresAName => (
                "--everything-instance requires a name",
                "--everything-instance 需要实例名称",
            ),
            Self::EverythingInstanceRequiresEverything => (
                "--everything-instance requires --everything",
                "--everything-instance 必须与 --everything 一起使用",
            ),
            Self::EverythingRequiresRecursiveOrTree => (
                "--everything requires --recursive or --tree",
                "--everything 必须与 --recursive 或 --tree 一起使用",
            ),
            Self::InvalidLanguage => (
                "--lang requires en, zh-CN or auto",
                "--lang 的值必须为 en、zh-CN 或 auto",
            ),
            Self::CommentFileRequired => (
                "--comment-file requires a file; prefix a filename starting with - with ./",
                "--comment-file 需要文件路径；以 - 开头的文件名请添加 ./ 前缀",
            ),
            Self::UnknownOrRepeatedOption => (
                "Unknown or repeated option; use -- before a path starting with -",
                "未知或重复的选项；路径以 - 开头时请在前面使用 --",
            ),
            Self::TreeAndLongAreMutuallyExclusive => (
                "--tree and --long are mutually exclusive",
                "--tree 和 --long 不能同时使用",
            ),
            Self::HelpUsage => ("Usage: dion help [command]", "用法：dion help [command]"),
            Self::VersionUsage => (
                "Version query only accepts --json and --lang alongside --version or -V",
                "版本查询仅允许 --version 或 -V 与 --json、--lang 组合",
            ),
            Self::UnknownHelpCommand => ("Unknown help command", "未知的帮助命令"),
            Self::ListOptionsOnly => (
                "--long, --recursive, --tree, --all, --color, --everything and --everything-instance are only supported by list",
                "--long、--recursive、--tree、--all、--color、--everything 和 --everything-instance 仅适用于 list 命令",
            ),
            Self::CommandUsage => (
                "Usage: dion [--json|-j] get <path> | remove <path> | list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--everything [--everything-instance <name>]] [--all|-a] [--color auto|always|never] | set <path> (<comment> | --stdin|-i | --comment-file|-f <file> | --edit|-e); use dion help for command aliases",
                "用法：dion [--json|-j] get <path> | remove <path> | list [directory] [--long|-l|--tree|-t] [--recursive|-r] [--everything [--everything-instance <name>]] [--all|-a] [--color auto|always|never] | set <path> (<comment> | --stdin|-i | --comment-file|-f <file> | --edit|-e)；使用 dion help 查看命令别名",
            ),
            Self::InvalidEntryName => (
                "Path must have a UTF-8 entry name",
                "路径中的条目名称必须是 UTF-8",
            ),
            Self::PathHasNoParent => ("Path has no parent", "路径没有上级目录"),
            Self::InvalidCommentEncoding => ("Comment must be UTF-8", "备注必须是 UTF-8"),
            Self::EditingRequiresATargetPath => {
                ("Editing requires a target path", "编辑备注需要指定目标路径")
            }
            Self::InvalidCommentInputEncoding => (
                "Comment input is not valid UTF-8",
                "备注输入不是有效的 UTF-8",
            ),
            Self::InvalidCommentBody => (
                "A nonblank comment without NUL or control character 04 is required",
                "备注不能为空或纯空白，也不能包含 NUL 或控制字符 04",
            ),
            Self::DescriptionFileContentChangedWhileEditing => (
                "Description file content changed while editing",
                "编辑期间备注文件内容发生变化",
            ),
            Self::CommentNotFound => ("Comment not found", "未找到备注"),
            Self::PathMustNotBeEmpty => ("Path must not be empty", "路径不能为空"),
            Self::InvalidEditorCommandLine => ("Invalid editor command line", "编辑器命令行无效"),
            #[cfg(not(windows))]
            Self::EditingCommentsRequiresWindows => {
                ("Editing comments requires Windows", "编辑备注需要 Windows")
            }
            #[cfg(not(windows))]
            Self::DeletingCommentsRequiresWindows => {
                ("Deleting comments requires Windows", "删除备注需要 Windows")
            }
            #[cfg(not(windows))]
            Self::WritingCommentsRequiresWindows => {
                ("Writing comments requires Windows", "写入备注需要 Windows")
            }
            Self::ParentIsNotADirectory => ("Parent is not a directory", "上级路径不是目录"),
            Self::InvalidDirectoryEncoding => (
                "Directory name is not valid UTF-8",
                "目录名称不是有效的 UTF-8",
            ),
            Self::MissingUtf8Bom => ("Missing UTF-8 BOM", "缺少 UTF-8 BOM"),
            Self::InvalidRecordEncoding => ("Record is not valid UTF-8", "记录不是有效的 UTF-8"),
            Self::PhysicalRecordTooLong => (
                "Physical record exceeds 4096 bytes",
                "备注记录超过 4096 字节",
            ),
            Self::UnclosedNameQuote => ("Unclosed name quote", "文件名引号未闭合"),
            Self::ExpectedASpaceAfterQuotedName => (
                "Expected a space after quoted name",
                "带引号的文件名后需要空格",
            ),
            Self::RecordNameIsEmpty => ("Record name is empty", "记录名称为空"),
            Self::DuplicateRecordName => (
                "Duplicate record name (case insensitive)",
                "记录名称重复（不区分大小写）",
            ),
            Self::UnknownExtension => (
                "Unknown extension; cannot modify this record",
                "程序扩展未知，无法修改此记录",
            ),
            Self::SerializedRecordTooLong => (
                "Serialized record exceeds 4096 bytes",
                "序列化后的备注记录超过 4096 字节",
            ),
            Self::FinalRecordTooLong => (
                "Final record exceeds 4096 bytes after adding CRLF",
                "添加 CRLF 后，末条记录超过 4096 字节",
            ),
            Self::DescriptionFileContentChangedBeforeCommit => (
                "Description file content changed before commit",
                "提交前备注文件内容发生变化",
            ),
            Self::DescriptionFileContentChangedBeforeDeletion => (
                "Description file content changed before deletion",
                "删除前备注文件内容发生变化",
            ),
            Self::DescriptionFileIsReadOnly => ("Description file is read-only", "备注文件为只读"),
            #[cfg(not(windows))]
            Self::EverythingRequiresWindows => (
                "Everything IPC is available only on Windows",
                "Everything IPC 仅在 Windows 上可用",
            ),
            Self::ModuleHandleFailed => (
                "Could not get the Dion module handle",
                "无法获取 Dion 模块句柄",
            ),
            Self::ReplyRegistrationFailed => (
                "Could not register the IPC reply window",
                "无法注册 IPC 回复窗口",
            ),
            Self::ReplyWindowFailed => (
                "Could not create the IPC reply window",
                "无法创建 IPC 回复窗口",
            ),
            Self::MajorVersionFailed => ("major version", "主版本查询失败"),
            Self::MinorVersionFailed => ("minor version", "次版本查询失败"),
            Self::RevisionFailed => ("revision", "修订版本查询失败"),
            Self::DatabaseReadinessFailed => ("database readiness", "数据库就绪状态查询失败"),
            Self::EverythingQueryTooLarge => (
                "Everything IPC query is too large",
                "Everything IPC 查询过大",
            ),
            Self::EverythingQueryRejected => (
                "Everything rejected the IPC query",
                "Everything 拒绝了 IPC 查询",
            ),
            Self::EverythingWaitFailed => (
                "Could not wait for the Everything reply",
                "无法等待 Everything 回复",
            ),
            Self::ReplyWindowClosed => ("The IPC reply window was closed", "IPC 回复窗口已关闭"),
            Self::InvalidEverythingReply => (
                "Everything returned an invalid result list",
                "Everything 返回了无效的结果列表",
            ),
        };
        if language.is_chinese() {
            chinese
        } else {
            english
        }
    }
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
