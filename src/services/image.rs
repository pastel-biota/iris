use std::{io::Cursor, sync::Arc};

use image::{ImageReader, Rgb};

use crate::{model::{Orientation, PhotoMeta, Properties}, services::image::resize::ResizeResult};

mod resize;
mod color;
mod stand;

pub struct ProcessResult {
    pub resized: Vec<ResizeResult>,
    pub averaged_color: Rgb<u8>,
}

pub async fn process_image_content(props: &Properties, original: &[u8]) -> anyhow::Result<ProcessResult> {
    tracing::info!("Decoding");
    let original = ImageReader::new(Cursor::new(original))
        .with_guessed_format()?
        .decode()?;
    tracing::info!("Finished decoding. Strarting resize");

    let orientation = if let Some(orientation) = props.orientation.as_ref() {
        orientation
    } else {
        &Orientation::default()
    };

    let stood = stand::stand_image(orientation, original.clone());
    let resized = resize::resize_images(stood).await?;
    let averaged_color = color::average_color(&resized.smallest_image);

    Ok(ProcessResult {
        resized: resized.resized,
        averaged_color,
    })
}

