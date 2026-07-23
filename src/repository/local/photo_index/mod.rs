pub mod date;
pub mod original_hash;
pub mod reference;

use anyhow::Context as _;

use crate::{
    model::{Identifier, ImageMeta, LocalIdentifier, PhotoReference},
    repository::{io::ScopedPath, photo_index::reference::ReferenceIndex},
};
pub trait PhotoIndexProvider {
    const INDEX_NAME: &'static str;
    type Entry: serde::Serialize + serde::de::DeserializeOwned + Default + std::fmt::Debug;

    fn load_to_file(&mut self, path: &ScopedPath) -> anyhow::Result<Self::Entry> {
        if !path.exists() {
            return self.init(path);
        }

        let bytes = path.read_binary().context(format!(
            "Failed to open a file for the {}",
            Self::INDEX_NAME
        ))?;

        serde_json::from_slice(&bytes)
            .context(format!("The {} contains invalid content", Self::INDEX_NAME))
    }

    fn init(&mut self, path: &ScopedPath) -> anyhow::Result<Self::Entry> {
        let value = Self::Entry::default();
        let bytes = serde_json::to_vec_pretty(&value).context(format!(
            "Failed to serialize an empty entry for the {}",
            Self::INDEX_NAME
        ))?;

        path.write(bytes).context(format!(
            "Failed to create a file for the {}",
            Self::INDEX_NAME
        ))?;

        Ok(value)
    }
}
