use std::io::Cursor;

use image::{DynamicImage, GenericImageView, ImageResult, imageops::FilterType, ImageFormat as SavingImageFormat};
use crate::{ingest::model::ImageMeta, processor::config::{ImageFormat, ResizeTargets}};

pub const RESIZE_TARGETS: [ResizeTargets; 4] = [
    // icon
    ResizeTargets {
        width: 64,
        height: 64,
        format: ImageFormat::WEBP,
    },
    // id: "thumbnail",
    ResizeTargets {
        width: 640,
        height: 640,
        format: ImageFormat::WEBP,
    },
    // id: "main",
    ResizeTargets {
        width: 1920,
        height: 1080,
        format: ImageFormat::WEBP,
    },
    // id: "highres",
    ResizeTargets {
        width: 2560,
        height: 1440,
        format: ImageFormat::PNG,
    },
];
pub const TINIEST_RESIZE_TARGET: &'static ResizeTargets = &RESIZE_TARGETS[0];

pub struct ResizeResult {
    pub target: ResizeTargets,
    pub meta: ImageMeta,
    pub image: DynamicImage,
    pub data: Vec<u8>,
}

pub struct Resized {
    pub resized: Vec<ResizeResult>,
}

// pub async fn resize_images(
//     original: DynamicImage,
//     target: Vec<&'static ResizeTargets>,
// ) -> anyhow::Result<Resized> {
//     let original = Arc::new(original);
//     let resized = tokio::task::spawn_blocking(move || {
//         target
//             .par_iter()
//             .map(|target| resize_image(target, original.clone()))
//             .collect::<Result<Vec<_>, _>>()
//     })
//     .await??;
// 
//     Ok(Resized { resized })
// }

pub fn resize_image(
    id: &str,
    target: ResizeTargets,
    original: &DynamicImage,
) -> ImageResult<ResizeResult> {
    let mut byte = Cursor::new(Vec::new());
    tracing::debug!("Resizing: {}", id);

    let (w, h) = determine_dimension(&target, original.dimensions());
    let ext = get_save_format(&target.format);

    let image = original.resize(w, h, FilterType::Gaussian);
    image.write_to(&mut byte, ext)?;
    tracing::debug!("Resized!: {}", id);

    ImageResult::Ok(ResizeResult {
        data: byte.into_inner(),
        meta: ImageMeta {
            width: image.width(),
            height: image.height(),
            extension: ext.extensions_str()[0].to_string(),
            mime: ext.to_mime_type().to_string(),
        },
        target,
        image,
    })
}

fn determine_dimension(target: &ResizeTargets, (width, height): (u32, u32)) -> (u32, u32) {
    let scale = f32::max(
        (target.height as f32) / height as f32,
        (target.width as f32) / width as f32,
    );

    (
        (width as f32 * scale) as u32,
        (height as f32 * scale) as u32,
    )
}

fn get_save_format(format: &ImageFormat) -> SavingImageFormat {
    match format {
        ImageFormat::PNG => SavingImageFormat::Png,
        ImageFormat::WEBP => SavingImageFormat::WebP,
        ImageFormat::JPEG => SavingImageFormat::Jpeg,
    }
}
