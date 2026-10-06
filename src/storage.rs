use crate::error::Error;
use std::{fs, io, path::Path};

pub fn read(file: &Path) -> Result<Option<Vec<u8>>, Error> {
    match fs::read(file) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // A missing parent is an access failure, not an absent comment file.
            let parent = file
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let metadata = fs::metadata(parent).map_err(|error| Error::io(error, file))?;
            if !metadata.is_dir() {
                return Err(Error::new("io_error", "Parent is not a directory", 1).at_file(file));
            }
            Ok(None)
        }
        Err(error) => Err(Error::io(error, file)),
    }
}
