mod local;
mod federated;
pub mod registry;

use std::collections::HashMap;

use crate::model::{Identifier, ImageMeta, PhotoMeta, PhotoReference};

pub use local::*;

// the trait with async fn is discouraged if it's public but this is for Iris only
// vis keyword "pub(crate)" marks that this is crate-local and suppresses the said warning
// (this compiler is smart wow)
pub(crate) trait ReadRegistry {
    async fn list_images(&mut self, beginning: Option<&Identifier>, size: usize) -> anyhow::Result<Vec<PhotoReference>>;
    async fn image_exists_with_hash(&mut self, hash: &str) -> anyhow::Result<bool>;
    async fn get_photos_list_by_hashes_list<'s, 'h>(
        &'s mut self,
        hash_to_lookup: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>>;
    async fn load_photo(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>>;
    async fn load_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<Vec<u8>>;
    async fn load_original_image(&mut self, photo_id: &Identifier) -> anyhow::Result<Vec<u8>>;
}

