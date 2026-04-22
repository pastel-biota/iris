use std::io::Cursor;

use anyhow::Context;
use bytes::Bytes;
use image::{DynamicImage, ImageFormat, ImageReader};

use crate::model::ImageMeta;

pub struct DecodeResult {
    pub image: DynamicImage,
    pub format: ImageFormat,
}

pub async fn decode_image(original_bytes: Bytes) -> anyhow::Result<DecodeResult> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let reader = ImageReader::new(Cursor::new(original_bytes)).with_guessed_format()?;
        let format = reader
            .format()
            .context("Could not guess the provided image's format")?;
        let image = reader.decode()?;

        Ok(DecodeResult { image, format })
    }).await.unwrap()
}

pub fn get_original_image_meta(
    original: &DynamicImage,
    format: &ImageFormat,
) -> anyhow::Result<ImageMeta> {
    Ok(ImageMeta {
        width: original.width(),
        height: original.height(),
        extension: format.extensions_str()
            .first()
            .with_context(|| format!("The provided image's guessed format with MIME Type '{}' does not have the extension to use", format.to_mime_type()))?
            .to_string(),
        mime: format.to_mime_type().to_string(),
    })
}
