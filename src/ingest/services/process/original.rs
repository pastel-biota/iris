use anyhow::Context;
use image::{DynamicImage, ImageFormat};

use crate::ingest::model::ImageMeta;

pub async fn get_original_image_meta(
    original: &DynamicImage,
    format: &ImageFormat,
) -> anyhow::Result<ImageMeta> {
    Ok(ImageMeta {
        width: original.width(),
        height: original.height(),
        extension: format.extensions_str()
            .get(0)
            .with_context(|| format!("The provided image's guessed format with MIME Type '{}' does not have the extension to use", format.to_mime_type()))?
            .to_string(),
        mime: format.to_mime_type().to_string(),
    })
}
