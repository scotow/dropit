use std::{
    io::{Error as IoError, ErrorKind},
    path::PathBuf,
};

use tokio::{fs, fs::File};

#[derive(Clone, Debug)]
pub struct Dir(PathBuf);

impl Dir {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self(path.into())
    }

    pub async fn create(&self, should_create: bool) -> Result<(), &'static str> {
        match File::open(&self.0).await {
            Ok(fd) => match fd.metadata().await {
                Ok(md) => {
                    if !md.is_dir() {
                        return Err("Uploads path is not a directory");
                    }
                }
                Err(_) => {
                    return Err("Cannot fetch uploads directory's metadata");
                }
            },
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    if should_create {
                        if tokio::fs::create_dir_all(&self.0).await.is_err() {
                            return Err("Cannot create uploads directory");
                        }
                    } else {
                        return Err("Uploads directory not found");
                    }
                } else {
                    return Err("Cannot open uploads directory");
                }
            }
        }
        Ok(())
    }

    fn file_path(&self, id: &str) -> PathBuf {
        self.0.join(id)
    }

    pub async fn create_file(&self, id: &str) -> Result<File, IoError> {
        File::create(self.file_path(id)).await
    }

    pub async fn open_file(&self, id: &str) -> Result<File, IoError> {
        File::open(self.file_path(id)).await
    }

    pub async fn delete_file(&self, id: &str) -> Result<(), IoError> {
        fs::remove_file(self.file_path(id)).await
    }
}
