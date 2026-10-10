use serde::Serialize;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::i18n::{self, Language};

#[derive(Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip)]
    pub exit_code: u8,
}

impl Error {
    pub fn write_text(&self, output: &mut impl Write, language: Language) -> io::Result<()> {
        write!(
            output,
            "{}: {}",
            self.code,
            i18n::error_message(self.code, &self.message, language)
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
            message: i18n::error_message(self.code, &self.message, language),
            file: self.file.as_deref(),
            line: self.line,
        }
    }

    pub fn new(code: &'static str, message: impl Into<String>, exit_code: u8) -> Self {
        Self {
            code,
            message: message.into(),
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

#[derive(serde::Serialize)]
pub struct LocalizedError<'a> {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<&'a Path>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}
