use std::{io::Cursor, ops::ControlFlow, sync::Arc};
use image::{DynamicImage, ImageReader, metadata::Orientation as OrientationMetadata};

use anyhow::bail;

use crate::{Context, model::{ImageMeta, Orientation, PhotoMeta, Rotation}, processor::{config::ResizeTargets, protocol::ImageProcessJob}, services::image::resize::ResizeResult};

#[tracing::instrument(level = "debug", skip_all, fields(
    photo_id = job.photo_id.to_string(),
    image_id = job.image_id,
))]
pub async fn run_image_processing(ctx: Arc<Context>, job: ImageProcessJob) -> anyhow::Result<()> {
    tracing::debug!("Processing job picked up, getting the photo");

    let Some(photo) = ({ ctx.registry.write().await.load_photo(&job.photo_id).await? }) else {
        bail!("No such photo found");
    };

    if let ControlFlow::Continue(reason) = check_already_processed(&photo, &job.target, &job.image_id) {
        tracing::debug!("The image will be processed: {reason}");
    } else {
        tracing::debug!("The image was already processed");
        return Ok(());
    }

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

fn check_already_processed(photo: &PhotoMeta, target: &ResizeTargets, image_id: &str) -> ControlFlow<(), &'static str> {
    tracing::trace!("Images: {}", photo.images.keys().map(|key| format!("{key}, ")).collect::<String>());
    let Some(processed) = photo.images.get(image_id) else {
        return ControlFlow::Continue("The photo does not contain image with the same ID");
    };

    tracing::trace!("{}x{} / {}x{}", processed.width, processed.height, target.width, target.height);
    if processed.width.abs_diff(target.width) > 10 && processed.height.abs_diff(target.height) > 10 {
        return ControlFlow::Continue("The size is not the same with the target");
    }

    return ControlFlow::Break(());
}

