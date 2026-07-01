//! `FileHandle` abstracts how a file's bytes are accessed. In M2+ this grows a
//! choice between memory-mapping (random-access analyzers: hex/entropy/carve)
//! and buffered streaming (sequential: strings/hashing). For M0 it just carries
//! the path.

use std::path::{Path, PathBuf};

use crate::error::CoreError;

pub struct FileHandle {
    path: PathBuf,
    size: u64,
}

impl FileHandle {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref().to_path_buf();
        let size = std::fs::metadata(&path)?.len();
        Ok(Self { path, size })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}
