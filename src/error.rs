use serde::Serialize;
use std::{
    io,
    path::{Path, PathBuf},
};

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
