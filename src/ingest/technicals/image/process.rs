use bytes::Bytes;
use chrono::{DateTime, FixedOffset};
use image::{DynamicImage, Rgb};

use crate::{
    model::{ImageMeta, Orientation, Properties},
    services::{
        extract::{color, exif},
        image::{decode, stand},
    },
};

pub use crate::services::hash::retrieve_file_hash as get_hash;

pub struct ProcessedImage {
    pub shot_time: DateTime<FixedOffset>,
    pub image_property: Properties,
    pub averaged_color: Rgb<u8>,
    pub original_meta: ImageMeta,
    pub original_image: DynamicImage,
}

pub async fn process_image(original_bytes: Bytes) -> anyhow::Result<ProcessedImage> {
    let exif_payloads = exif::read_exif(&original_bytes).await?;

    let decoded = decode::decode_image(original_bytes.clone()).await?;
    tracing::info!("Finished decoding. Starting resize");

    let orientation = match exif_payloads.props.orientation.as_ref() {
        Some(orientation) => orientation,
        None => &Orientation::default(),
    };

    let original_meta = decode::get_original_image_meta(&decoded.image, &decoded.format)?;
    let stood = stand::stand_image(orientation, decoded.image).await;
    let averaged_color = color::average_color(&stood);

    Ok(ProcessedImage {
        shot_time: exif_payloads.shot_time,
        image_property: exif_payloads.props,
        averaged_color,
        original_meta,
        original_image: stood,
    })
}
