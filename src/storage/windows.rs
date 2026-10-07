use crate::error::Error;
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
    ptr,
};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ReplaceFileW(
        replaced: *const u16,
        replacement: *const u16,
        backup: *const u16,
        flags: u32,
        exclude: *mut std::ffi::c_void,
        reserved: *mut std::ffi::c_void,
    ) -> i32;
    fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    fn SetFileAttributesW(file: *const u16, attributes: u32) -> i32;
}

/// Commits through a complete same-directory temporary file, without retry or fallback.
/// Content checks detect some conflicts; this remains a single-writer operation.
pub fn commit(file: &Path, original: Option<&[u8]>, bytes: &[u8]) -> Result<(), Error> {
    let parent = file
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|error| Error::io(error, file))?;
    let destination = parent.join("descript.ion");
    // Deny writes while preparing the replacement, but allow ReplaceFileW to delete/rename.
    let mut source = match original {
        Some(_) => Some(
            OpenOptions::new()
                .read(true)
                .share_mode(1 | 4)
                .open(&destination)
                .map_err(|error| Error::io(error, file))?,
        ),
        None => None,
    };
    let attributes = if let Some(source) = &mut source {
        let metadata = source.metadata().map_err(|error| Error::io(error, file))?;
        if metadata.permissions().readonly() {
            return Err(Error::new("io_error", "Description file is read-only", 1).at_file(file));
        }
        let mut current = Vec::new();
        source
            .read_to_end(&mut current)
            .map_err(|error| Error::io(error, file))?;
        if Some(current.as_slice()) != original {
            return Err(Error::new(
                "content_changed",
                "Description file content changed before commit",
                1,
            )
            .at_file(file));
        }
        metadata.file_attributes()
    } else {
        2 // FILE_ATTRIBUTE_HIDDEN
    };
    let temporary = tempfile::Builder::new()
        .prefix(".dion-")
        .suffix(".tmp")
        .tempfile_in(&parent)
        .map_err(|error| Error::io(error, file))?;
    // Retain the temporary file on any subsequent I/O failure, including partial writes.
    let (mut output, recovery) = temporary
        .keep()
        .map_err(|error| Error::io(error.error, file))?;
    let result = (|| -> io::Result<()> {
        output.write_all(bytes)?;
        output.sync_all()?;
        drop(output);
        // A rename can bypass the write-sharing guard. Check the current path again
        // before replacing it; the remaining check/replace race is outside our guarantee.
        if super::read(&destination)
            .map_err(|error| io::Error::other(error.message))?
            .as_deref()
            != original
        {
            return Err(io::Error::other(
                "Description file content changed before commit",
            ));
        }
        let replacement: Vec<_> = recovery.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<_> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // SAFETY: Both paths are live NUL-terminated UTF-16 buffers. Reserved arguments
        // are null, and zero flags require ACL merge errors to fail rather than be ignored.
        let success = unsafe {
            if original.is_some() {
                ReplaceFileW(
                    destination.as_ptr(),
                    replacement.as_ptr(),
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            } else {
                if SetFileAttributesW(replacement.as_ptr(), attributes) == 0 {
                    return Err(io::Error::last_os_error());
                }
                // Zero flags refuse to overwrite a file created since our initial read.
                MoveFileExW(replacement.as_ptr(), destination.as_ptr(), 0)
            }
        };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        if original.is_some() {
            // ReplaceFile preserves creation time and DACLs. Restore the ordinary file
            // attributes explicitly; compression/encryption are preserved by ReplaceFile.
            // SAFETY: destination remains a live NUL-terminated UTF-16 path.
            if unsafe { SetFileAttributesW(destination.as_ptr(), attributes) } == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    })();
    result.map_err(|error| Error::new("io_error", format!(
        "Commit failed: {error}. Inspect description file {} and temporary file {} for recovery; replacement may be partially complete",
        parent.join("descript.ion").display(), recovery.display()), 1).at_file(file))
}
