use std::{io::Cursor, sync::Arc};
use image::{DynamicImage, ImageReader, metadata::Orientation as OrientationMetadata};

use anyhow::bail;

use crate::{Context, ingest::model::{Orientation, Rotation}, processor::protocol::ImageProcessJob};

pub async fn run_image_processing(ctx: Arc<Context>, job: ImageProcessJob) -> anyhow::Result<()> {
    let _span = tracing::debug_span!("Image Processing", photo_id = job.photo_id.to_string(), image_id=&job.image_id);

    tracing::debug!("Processing job picked up, getting the photo");

    let Some(photo) = ({ ctx.registry.write().await.load_photo(&job.photo_id)? }) else {
        bail!("No such photo found");
    };

    let original_image = { ctx.registry.write().await.load_original_image(&job.photo_id).await? };

    tracing::debug!("Reading the image");
    let original_image = ImageReader::new(Cursor::new(original_image))
        .with_guessed_format()?
        .decode()?;

    tracing::debug!("Rotating the image upright");
    let orientation = match photo.properties.orientation.as_ref() {
        Some(orientation) => orientation,
        None => &Orientation::default(),
    };

    let original_image = stand_image(orientation, original_image);

    tracing::debug!("Resizing");
    let result = super::image::resize::resize_image(&job.image_id, job.target, &original_image)?;

    tracing::debug!("Registering");
    let mut registry = ctx.registry.write().await;
    registry.upload_image(&job.photo_id, &job.image_id, &result.meta, &result.data).await?;

    tracing::debug!("The image '{}' has been processed", &job.image_id);

    Ok(())
}

fn stand_image(original_orientation: &Orientation, mut image: DynamicImage) -> DynamicImage {
    // image.apply_orientation(/**/) rotates N deg in clockwise
    let orientation_meta = match &original_orientation.rotation {
        Rotation::Upright => OrientationMetadata::NoTransforms,
        Rotation::UpsideDown => OrientationMetadata::Rotate180,
        Rotation::CounterClockwise => OrientationMetadata::Rotate270,
        Rotation::Clockwise => OrientationMetadata::Rotate90,
    };

    image.apply_orientation(orientation_meta);
    image
}
