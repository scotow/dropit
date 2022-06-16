use std::io::ErrorKind;
use std::path::PathBuf;
use tokio::fs::File;

#[derive(Clone, Debug)]
pub struct Dir(PathBuf);

impl Dir {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self(path.into())
    }

    pub fn file_path(&self, id: &str) -> PathBuf {
        self.0.join(id)
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
                        if let Err(_) = tokio::fs::create_dir_all(&self.0).await {
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
}
