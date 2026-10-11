use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::i18n::Language;

/// Program diagnostics have stable identities independent of their wording.
#[derive(Clone, Copy)]
pub enum Diagnostic {
    EverythingInstanceNotFound,
    EverythingDatabaseIsNotReady,
    EverythingIpcTimedOut,
    UnsupportedEverythingVersion,
    InvalidColor,
    EverythingInstanceRequiresAName,
    EverythingInstanceRequiresEverything,
    EverythingRequiresRecursiveOrTree,
    InvalidLanguage,
    CommentFileRequired,
    UnknownOrRepeatedOption,
    TreeAndLongAreMutuallyExclusive,
    HelpUsage,
    UnknownHelpCommand,
    ListOptionsOnly,
    CommandUsage,
    InvalidEntryName,
    PathHasNoParent,
    InvalidCommentEncoding,
    EditingRequiresATargetPath,
    InvalidCommentInputEncoding,
    InvalidCommentBody,
    DescriptionFileContentChangedWhileEditing,
    CommentNotFound,
    PathMustNotBeEmpty,
    InvalidEditorCommandLine,
    #[cfg(not(windows))]
    EditingCommentsRequiresWindows,
    #[cfg(not(windows))]
    DeletingCommentsRequiresWindows,
    #[cfg(not(windows))]
    WritingCommentsRequiresWindows,
    ParentIsNotADirectory,
    InvalidDirectoryEncoding,
    MissingUtf8Bom,
    InvalidRecordEncoding,
    PhysicalRecordTooLong,
    UnclosedNameQuote,
    ExpectedASpaceAfterQuotedName,
    RecordNameIsEmpty,
    DuplicateRecordName,
    UnknownExtension,
    SerializedRecordTooLong,
    FinalRecordTooLong,
    DescriptionFileContentChangedBeforeCommit,
    DescriptionFileContentChangedBeforeDeletion,
    DescriptionFileIsReadOnly,
    #[cfg(not(windows))]
    EverythingRequiresWindows,
    ModuleHandleFailed,
    ReplyRegistrationFailed,
    ReplyWindowFailed,
    MajorVersionFailed,
    MinorVersionFailed,
    RevisionFailed,
    DatabaseReadinessFailed,
    EverythingQueryTooLarge,
    EverythingQueryRejected,
    EverythingWaitFailed,
    ReplyWindowClosed,
    InvalidEverythingReply,
}

pub struct Error {
    pub code: &'static str,
    pub message: Box<Message>,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub exit_code: u8,
}

impl Error {
    pub fn write_text(&self, output: &mut impl Write, language: Language) -> io::Result<()> {
        write!(
            output,
            "{}: {}",
            self.code,
            self.message.render(language, self.code)
        )?;
        if let Some(file) = &self.file {
            write!(output, " ({})", file.display())?;
        }
        if let Some(line) = self.line {
            if language.is_chinese() {
                write!(output, " 第 {line} 行")?;
            } else {
                write!(output, " at line {line}")?;
            }
        }
        writeln!(output)
    }

    pub fn localized(&self, language: Language) -> LocalizedError<'_> {
        LocalizedError {
            code: self.code,
            message: self.message.render(language, self.code),
            file: self.file.as_deref(),
            line: self.line,
        }
    }

    pub fn new(code: &'static str, message: impl Into<Message>, exit_code: u8) -> Self {
        Self {
            code,
            message: Box::new(message.into()),
            file: None,
            line: None,
            exit_code,
        }
    }

    pub fn at_file(mut self, file: &Path) -> Self {
        self.file = Some(file.to_owned());
        self
    }

    pub fn io(error: io::Error, file: &Path) -> Self {
        Self::new("io_error", error.to_string(), 1).at_file(file)
    }
}

/// Keeps system details and recovery paths separate from translated program text.
pub enum Message {
    Diagnostic(Diagnostic),
    System(String),
    EditorLaunch(String),
    EditorExit(String),
    Windows {
        context: Diagnostic,
        code: u32,
    },
    Commit {
        detail: Box<Message>,
        description: PathBuf,
        temporary: PathBuf,
    },
    RetainedEdit {
        source: Box<Message>,
        text: PathBuf,
    },
}

impl From<Diagnostic> for Message {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Diagnostic(diagnostic)
    }
}

impl From<String> for Message {
    fn from(detail: String) -> Self {
        Self::System(detail)
    }
}

impl Message {
    fn render(&self, language: Language, code: &str) -> String {
        let chinese = language.is_chinese();
        match self {
            Self::Diagnostic(diagnostic) => diagnostic.text(language).to_owned(),
            Self::System(detail) if chinese && code == "io_error" => {
                format!("文件操作失败：{detail}")
            }
            Self::System(detail) => detail.clone(),
            Self::EditorLaunch(detail) if chinese => format!("无法启动编辑器：{detail}"),
            Self::EditorLaunch(detail) => format!("Could not run editor: {detail}"),
            Self::EditorExit(detail) if chinese => format!("编辑器异常退出：{detail}"),
            Self::EditorExit(detail) => format!("Editor exited unsuccessfully: {detail}"),
            Self::Windows { context, code } => {
                format!("{} (Windows error {code})", context.text(language))
            }
            Self::Commit {
                detail,
                description,
                temporary,
            } if chinese => format!(
                "提交备注失败：{}。检查备注文件 {} 和临时文件 {} 以恢复；替换可能只完成了一部分",
                detail.render(language, ""),
                description.display(),
                temporary.display()
            ),
            Self::Commit {
                detail,
                description,
                temporary,
            } => format!(
                "Commit failed: {}. Inspect description file {} and temporary file {} for recovery; replacement may be partially complete",
                detail.render(language, ""),
                description.display(),
                temporary.display()
            ),
            Self::RetainedEdit { source, text } if chinese => format!(
                "{}；编辑文本已保留在 {}",
                source.render(language, code),
                text.display()
            ),
            Self::RetainedEdit { source, text } => format!(
                "{}; Edit text retained at {}",
                source.render(language, code),
                text.display()
            ),
        }
    }
}

#[derive(serde::Serialize)]
pub struct LocalizedError<'a> {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<&'a Path>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}
