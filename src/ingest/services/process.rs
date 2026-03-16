mod color;
mod exif;
mod hash;
mod original;
mod stand;

use std::io::Cursor;

use anyhow::Context as _;
use chrono::{DateTime, FixedOffset};
use image::{DynamicImage, ImageReader, Rgb};

use crate::ingest::{
    model::{ImageMeta, Orientation, Properties},
    services::resize::{ResizeResult, TINIEST_RESIZE_TARGET},
};

pub struct ProcessedImage {
    pub shot_time: DateTime<FixedOffset>,
    pub image_property: Properties,
    pub averaged_color: Rgb<u8>,
    pub original_meta: ImageMeta,
    pub original_image: DynamicImage,
    pub instant_image: ResizeResult,
}
pub fn get_hash(original_bytes: &[u8]) -> String {
    hash::retrieve_file_hash(&original_bytes)
}

pub async fn process_image(original_bytes: &[u8]) -> anyhow::Result<ProcessedImage> {
    let exif_payloads = exif::read_exif(&original_bytes).await?;

    let reader = ImageReader::new(Cursor::new(original_bytes)).with_guessed_format()?;
    let format = reader
        .format()
        .context("Could not guess the provided image's format")?;

    let original = reader.decode()?;
    tracing::info!("Finished decoding. Strarting resize");

    let orientation = match exif_payloads.props.orientation.as_ref() {
        Some(orientation) => orientation,
        None => &Orientation::default(),
    };

    let original_meta = original::get_original_image_meta(&original, &format).await?;
    let stood = stand::stand_image(orientation, original);
    let instant_image = super::resize::resize_images(stood.clone(), vec![TINIEST_RESIZE_TARGET])
        .await?
        .resized
        .into_iter()
        .next()
        .expect("The image is to be resized but it was not");
    let averaged_color = color::average_color(&instant_image.image);

    Ok(ProcessedImage {
        shot_time: exif_payloads.shot_time,
        image_property: exif_payloads.props,
        averaged_color,
        original_meta,
        original_image: stood,
        instant_image,
    })
}
