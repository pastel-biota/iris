#[allow(clippy::disallowed_types)]
use std::{path::{Path, PathBuf}};

use tokio::fs::File;

#[derive(Clone, Debug)]
#[allow(clippy::disallowed_types)]
pub struct ScopedPath {
    allowed_path: PathBuf,
    path: PathBuf,
}

#[allow(clippy::disallowed_types)]
impl ScopedPath {
    pub fn from_allowed_dir(dir: &Path) -> ScopedPath {
        let dir = dir.canonicalize().unwrap();
        if !dir.is_dir() {
            panic!("ScopedPath::from_allowed_dir was invoked for non-directory path");
        }

        tracing::debug!("Initialized ScopedPath at {}", dir.display());

        Self {
            allowed_path: dir.to_path_buf(),
            path: dir.to_path_buf(),
        }
    }

    pub fn join(&self, component: impl AsRef<Path>) -> ScopedPath {
        self.with_new_path(self.path.join(component))
    }

    pub fn use_path(&self) -> &Path {
        &self.path
    }

    pub fn read_binary(&self) -> std::io::Result<Vec<u8>> {
        std::fs::read(&self.path)
    }

    pub fn create_dir_all(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.path)
    }

    pub async fn create_file(&self) -> std::io::Result<File> {
        tokio::fs::File::create(&self.path).await
    }

    pub async fn open_file(&self) -> std::io::Result<File> {
        tokio::fs::File::open(&self.path).await
    }

    pub async fn remove_file(&self) -> std::io::Result<()> {
        tokio::fs::remove_file(&self.path).await
    }

    pub fn write(&self, array: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(&self.path, array)
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn display(&self) -> std::path::Display<'_> {
        self.path.display()
    }

    pub fn parent(&self) -> Option<ScopedPath> {
        self.path.parent().map(|parent| self.with_new_path(parent))
    }

    fn with_new_path(&self, new_path: impl AsRef<Path>) -> Self {
        let new_path = new_path.as_ref();

        if new_path.components().any(|component| component.as_os_str() == "..") {
            panic!("Path contains reference to the parent dir: {}", new_path.display());
        }

        if std::path::absolute(new_path).map(|path| !path.starts_with(&self.allowed_path)).unwrap_or(true) {
            panic!("Path resolves to invalid location: {}", new_path.display());
        }

        if new_path.exists() {
            let canonicalized = new_path.canonicalize().unwrap();

            if !canonicalized.starts_with(&self.allowed_path) {
                panic!("Path resolves to invalid location:\n    Specified: {}\n    Resolved: {}", new_path.display(), canonicalized.display());
            }
        }

        tracing::trace!(
            new_path = new_path.display().to_string(),
            allowed_path = self.allowed_path.display().to_string(),
            "New scoped path",
        );

        Self {
            allowed_path: self.allowed_path.clone(),
            path: new_path.to_path_buf(),
        }
    }
}

