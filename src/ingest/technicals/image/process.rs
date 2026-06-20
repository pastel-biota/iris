use bytes::Bytes;
use chrono::{DateTime, FixedOffset};
use image::{Rgb};

use crate::{
    model::{ImageMeta, Properties},
    services::{
        extract::{color, exif},
        image::decode,
    },
};

pub use crate::services::hash::retrieve_file_hash as get_hash;

pub struct ProcessedImage {
    pub shot_time: DateTime<FixedOffset>,
    pub image_property: Properties,
    pub averaged_color: Rgb<u8>,
    pub original_meta: ImageMeta,
}

pub async fn process_image(original_bytes: Bytes) -> anyhow::Result<ProcessedImage> {
    let exif_payloads = exif::read_exif(&original_bytes).await?;

    let decoded = decode::decode_image(original_bytes.clone()).await?;

    let original_meta = decode::get_original_image_meta(&decoded.image, &decoded.format)?;
    let averaged_color = color::average_color(&decoded.image);

    Ok(ProcessedImage {
        shot_time: exif_payloads.shot_time,
        image_property: exif_payloads.props,
        averaged_color,
        original_meta,
    })
}
