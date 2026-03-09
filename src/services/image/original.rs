use anyhow::Context;
use image::{DynamicImage, ImageFormat};

pub struct OriginalImage {
    pub w: u32,
    pub h: u32,
    pub ext: &'static str,
    pub mime: &'static str,
}

pub async fn get_original_image_meta(
    original: &DynamicImage,
    format: &ImageFormat,
) -> anyhow::Result<OriginalImage> {
    Ok(OriginalImage {
        w: original.width(),
        h: original.height(),
        ext: format.extensions_str().get(0)
            .with_context(|| format!("The provided image's guessed format with MIME Type '{}' does not have the extension to use", format.to_mime_type()))?,
        mime: format.to_mime_type(),
    })
}

