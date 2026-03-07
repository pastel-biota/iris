use image::DynamicImage;

use crate::model::{Orientation, Rotation};

pub fn stand_image(original_orientation: &Orientation, image: DynamicImage) -> DynamicImage {
    // image.rotateN() rotates N deg in clockwise
    match &original_orientation.rotation {
        Rotation::Upright => image,
        Rotation::UpsideDown => image.rotate180(),
        Rotation::CounterClockwise => image.rotate270(),
        Rotation::Clockwise => image.rotate90(),
    }
}

