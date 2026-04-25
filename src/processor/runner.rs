use std::{io::Cursor, sync::Arc};
use image::{DynamicImage, ImageReader, metadata::Orientation as OrientationMetadata};

use anyhow::bail;

use crate::{Context, model::{Orientation, Rotation}, processor::protocol::ImageProcessJob, services::image::resize::ResizeResult};

#[tracing::instrument(level = "debug", skip_all, fields(
    photo_id = job.photo_id.to_string(),
    image_id = job.image_id,
))]
pub async fn run_image_processing(ctx: Arc<Context>, job: ImageProcessJob) -> anyhow::Result<()> {
    tracing::debug!("Processing job picked up, getting the photo");

    let Some(photo) = ({ ctx.registry.write().await.load_photo(&job.photo_id)? }) else {
        bail!("No such photo found");
    };

    let original_image = { ctx.registry.write().await.load_original_image(&job.photo_id).await? };

    let result = tokio::task::spawn_blocking({
        let job = job.clone();
        let span = tracing::Span::current();
        move || -> anyhow::Result<ResizeResult> {
            let _span = span.enter();
            tracing::debug!("reading the image");
            let original_image = ImageReader::new(Cursor::new(original_image))
                .with_guessed_format()?
                .decode()?;

            tracing::debug!("rotating the image upright");
            let orientation = match photo.properties.orientation.as_ref() {
                Some(orientation) => orientation,
                None => &Orientation::default(),
            };

            let original_image = stand_image(orientation, original_image);

            tracing::debug!("resizing");
            let result = crate::services::image::resize::resize_image(&job.image_id, job.target, &original_image)?;

            Ok(result)
        }
    }).await.unwrap()?;

    tracing::debug!("uploading to the registry");
    
    {
        let mut registry = ctx.registry.write().await;
        registry.upload_image(&job.photo_id, &job.image_id, &result.meta, &result.data).await?;
    }

    tracing::debug!("image processing is done!");

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
