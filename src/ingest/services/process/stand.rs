use image::{DynamicImage, metadata::Orientation as OrientationMetadata};

use crate::ingest::model::{Orientation, Rotation};

pub fn stand_image(original_orientation: &Orientation, mut image: DynamicImage) -> DynamicImage {
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
