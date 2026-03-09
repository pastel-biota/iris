use std::io::Cursor;

use anyhow::Context as _;
use image::{ImageReader, Rgb};

use crate::{model::{Orientation, Properties}, services::image::{original::OriginalImage, resize::ResizeResult}};

mod color;
mod original;
mod resize;
mod stand;

pub struct ProcessResult {
    pub resized: Vec<ResizeResult>,
    pub averaged_color: Rgb<u8>,
    pub original_meta: OriginalImage,
}

pub async fn process_image_content(props: &Properties, original: &[u8]) -> anyhow::Result<ProcessResult> {
    tracing::info!("Decoding");
    let reader = ImageReader::new(Cursor::new(original))
        .with_guessed_format()?;
    let format = reader.format()
        .context("Could not guess the provided image's format")?;

    let original = reader.decode()?;
    tracing::info!("Finished decoding. Strarting resize");

    let orientation = if let Some(orientation) = props.orientation.as_ref() {
        orientation
    } else {
        &Orientation::default()
    };

    let original_meta = original::get_original_image_meta(&original, &format).await?;
    let stood = stand::stand_image(orientation, original);
    let resized = resize::resize_images(stood).await?;
    let averaged_color = color::average_color(&resized.smallest_image);

    Ok(ProcessResult {
        resized: resized.resized,
        averaged_color,
        original_meta,
    })
}

