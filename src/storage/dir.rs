use std::path::PathBuf;

pub struct Dir(PathBuf);

impl Dir {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self(path.into())
    }

    pub fn file_path(&self, id: &str) -> PathBuf {
        self.0.join(id)
    }
}