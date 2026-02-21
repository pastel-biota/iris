mod exif;
mod hash;

use chrono::{DateTime, FixedOffset};

use crate::model::Properties;

#[derive(Clone, Debug)]
pub struct ProcessedImage {
    pub shot_time: DateTime<FixedOffset>,
    pub image_property: Properties,
    pub sha256: String,
}

pub async fn process_image(bytes: &[u8]) -> anyhow::Result<ProcessedImage> {
    let exif_payloads = exif::read_exif(&bytes).await?;

    Ok(ProcessedImage {
        shot_time: exif_payloads.shot_time,
        image_property: exif_payloads.props,
        sha256: hash::retrieve_file_hash(&bytes),
    })
}
