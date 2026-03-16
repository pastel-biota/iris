use std::{io::Cursor, sync::Arc};

use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult, imageops::FilterType};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator};

use crate::ingest::model::ImageMeta;

pub struct ResizeTargets {
    pub w: u32,
    pub h: u32,
    pub id: &'static str,
    pub ext: ImageFormat,
}

pub const RESIZE_TARGETS: [ResizeTargets; 4] = [
    ResizeTargets {
        w: 64,
        h: 64,
        id: "icon",
        ext: ImageFormat::WebP,
    },
    ResizeTargets {
        w: 640,
        h: 640,
        id: "thumbnail",
        ext: ImageFormat::WebP,
    },
    ResizeTargets {
        w: 1920,
        h: 1080,
        id: "main",
        ext: ImageFormat::WebP,
    },
    ResizeTargets {
        w: 2560,
        h: 1440,
        id: "highres",
        ext: ImageFormat::Png,
    },
];
pub const TINIEST_RESIZE_TARGET: &'static ResizeTargets = &RESIZE_TARGETS[0];

pub struct ResizeResult {
    pub target: &'static ResizeTargets,
    pub meta: ImageMeta,
    pub image: DynamicImage,
    pub data: Vec<u8>,
}

pub struct Resized {
    pub resized: Vec<ResizeResult>,
}

pub async fn resize_images(
    original: DynamicImage,
    target: Vec<&'static ResizeTargets>,
) -> anyhow::Result<Resized> {
    let original = Arc::new(original);
    let resized = tokio::task::spawn_blocking(move || {
        target
            .par_iter()
            .map(|target| resize_image(target, original.clone()))
            .collect::<Result<Vec<_>, _>>()
    })
    .await??;

    Ok(Resized { resized })
}

fn resize_image(
    target: &'static ResizeTargets,
    original: Arc<DynamicImage>,
) -> ImageResult<ResizeResult> {
    let mut byte = Cursor::new(Vec::new());
    tracing::debug!("Resizing: {}", target.id);

    let (w, h) = determine_dimension(target, original.dimensions());

    let image = original.resize(w, h, FilterType::Gaussian);
    image.write_to(&mut byte, target.ext)?;
    tracing::debug!("Resized!: {}", target.id);

    ImageResult::Ok(ResizeResult {
        data: byte.into_inner(),
        meta: ImageMeta {
            width: image.width(),
            height: image.height(),
            extension: target.ext.extensions_str()[0].to_string(),
            mime: target.ext.to_mime_type().to_string(),
        },
        target,
        image,
    })
}

fn determine_dimension(target: &ResizeTargets, (width, height): (u32, u32)) -> (u32, u32) {
    let scale = f32::max(
        (target.h as f32) / height as f32,
        (target.w as f32) / width as f32,
    );

    (
        (width as f32 * scale) as u32,
        (height as f32 * scale) as u32,
    )
}
