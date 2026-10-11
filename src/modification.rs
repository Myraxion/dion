use crate::{
    comment, editor,
    error::{Diagnostic, Error},
    name::Name,
    storage,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// An entry's comment in its parent's description file, without a disk snapshot.
pub struct Target<'a> {
    path: &'a Path,
    name: &'a str,
    file: PathBuf,
}

impl<'a> Target<'a> {
    /// Validates the entry name without I/O, before the caller reads comment input.
    /// The caller resolves the entry path lexically, without following its link.
    pub fn new(path: &'a Path) -> Result<Self, Error> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| Error::new("invalid_argument", Diagnostic::InvalidEntryName, 2))?;
        let file = path
            .parent()
            .ok_or_else(|| Error::new("invalid_argument", Diagnostic::PathHasNoParent, 2))?
            .join("descript.ion");
        Ok(Self { path, name, file })
    }

    /// Validates the body and target before reading the description file.
    /// Returns whether bytes changed; a valid no-op does not write the file.
    pub fn set(&self, body: &str) -> Result<bool, Error> {
        let body = self.validate_set(body)?;
        let original = storage::read(&self.file)?;
        self.set_record(original.as_deref(), &body)
    }

    /// Allows orphan removal and deletes the description file with its last record.
    pub fn remove(&self) -> Result<bool, Error> {
        let original = storage::read(&self.file)?;
        self.remove_record(original.as_deref())
    }

    /// Edits against the original snapshot, including conflict checks on no-op edits.
    /// Empty text removes the record; failed edits retain their recovery text.
    pub fn edit(&self) -> Result<bool, Error> {
        let original = storage::read(&self.file)?;
        let records = original
            .as_deref()
            .map(comment::parse)
            .transpose()
            .map_err(|error| error.at_file(&self.file))?
            .unwrap_or_default();
        let key = Name::new(self.name);
        let prefill = records
            .iter()
            .find(|record| key.matches(record.name))
            .map_or("", |record| record.comment.as_ref());
        let text = editor::create(prefill)?;
        let result = (|| {
            editor::run(&text)?;
            // Re-read only to check for conflicts, even on no-op edits. The later
            // commit must still use the snapshot captured before editing.
            if storage::read(&self.file)? != original {
                return Err(Error::new(
                    "content_changed",
                    Diagnostic::DescriptionFileContentChangedWhileEditing,
                    1,
                )
                .at_file(&self.file));
            }
            let bytes = fs::read(&text).map_err(|error| Error::io(error, &text))?;
            let body = comment::decode_input(&bytes).map_err(|error| error.at_file(&text))?;
            let changed = if body.is_empty() {
                self.remove_record(original.as_deref())?
            } else {
                let body = self.validate_set(&body)?;
                self.set_record(original.as_deref(), &body)?
            };
            fs::remove_file(&text).map_err(|error| Error::io(error, &text))?;
            Ok(changed)
        })();
        result.map_err(|error| editor::recovery(error, &text))
    }

    fn validate_set(&self, body: &str) -> Result<String, Error> {
        if body.trim().is_empty() || body.contains(['\0', '\u{4}']) {
            return Err(Error::new(
                "invalid_argument",
                Diagnostic::InvalidCommentBody,
                2,
            ));
        }
        let body = body.replace("\r\n", "\n").replace('\r', "\n");
        fs::symlink_metadata(self.path).map_err(|error| Error::io(error, self.path))?;
        Ok(body)
    }

    fn set_record(&self, original: Option<&[u8]>, body: &str) -> Result<bool, Error> {
        let bytes =
            comment::set(original, self.name, body).map_err(|error| error.at_file(&self.file))?;
        if let Some(bytes) = &bytes {
            storage::commit(&self.file, original, bytes)?;
        }
        Ok(bytes.is_some())
    }

    fn remove_record(&self, original: Option<&[u8]>) -> Result<bool, Error> {
        let Some(original) = original else {
            return Ok(false);
        };
        match comment::remove(original, self.name).map_err(|error| error.at_file(&self.file))? {
            comment::Removal::Unchanged => Ok(false),
            comment::Removal::Update(bytes) => {
                storage::commit(&self.file, Some(original), &bytes)?;
                Ok(true)
            }
            comment::Removal::DeleteFile => {
                storage::remove(&self.file, original)?;
                Ok(true)
            }
        }
    }
}
