use std::{io::Cursor, sync::Arc};

use image::{ImageReader, Rgb};

use crate::services::image::resize::ResizeResult;

mod resize;
mod color;

pub struct ProcessResult {
    pub resized: Vec<ResizeResult>,
    pub averaged_color: Rgb<u8>,
}

pub async fn process_image_content(original: &[u8]) -> anyhow::Result<ProcessResult> {
    tracing::info!("Decoding");
    let original = ImageReader::new(Cursor::new(original))
        .with_guessed_format()?
        .decode()?;
    tracing::info!("Finished decoding. Strarting resize");

    let original = Arc::new(original);

    let resized = resize::resize_images(original.clone()).await?;
    let averaged_color = color::average_color(&resized.smallest_image);

    Ok(ProcessResult {
        resized: resized.resized,
        averaged_color,
    })
}

