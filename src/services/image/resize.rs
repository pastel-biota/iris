use std::{io::Cursor, sync::Arc};

use image::{DynamicImage, ImageFormat, ImageResult, imageops::FilterType};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator};

pub struct ResizeTargets {
    pub w: u32,
    pub h: u32,
    pub id: &'static str,
    pub ext: ImageFormat,
}

pub const RESIZE_TARGETS: [ResizeTargets; 4] = [
    ResizeTargets { w: 64, h: 64, id: "icon", ext: ImageFormat::WebP },
    ResizeTargets { w: 640, h: 640, id: "thumbnail", ext: ImageFormat::WebP },
    ResizeTargets { w: 1920, h: 1080, id: "main", ext: ImageFormat::WebP },
    ResizeTargets { w: 2560, h: 1440, id: "highres", ext: ImageFormat::Png },
];

pub struct ResizeResult {
    pub target: &'static ResizeTargets,
    pub data: Vec<u8>,
}

pub struct Resized {
    pub resized: Vec<ResizeResult>,
    pub smallest_image: DynamicImage,
}

pub async fn resize_images(original: DynamicImage) -> anyhow::Result<Resized> {
    let original = Arc::new(original);
    let resized = tokio::task::spawn_blocking(move || {
        RESIZE_TARGETS
            .par_iter()
            .map(|target| {
                let mut byte = Cursor::new(Vec::new());
                tracing::debug!("Resizing: {}", target.id);
                let image = original
                    .resize(target.w, target.h, FilterType::Gaussian);
                image.write_to(&mut byte, target.ext)?;
                tracing::debug!("Resized!: {}", target.id);
                ImageResult::Ok((
                    ResizeResult {
                        target,
                        data: byte.into_inner()
                    },
                    image,
                ))
            })
            .collect::<Result<Vec<_>, _>>()
    }).await??;

    let (resized, mut images): (Vec<ResizeResult>, Vec<DynamicImage>) = resized.into_iter().unzip();

    let smallest_idx = resized.iter()
        .enumerate()
        .min_by_key(|(_, content)| content.data.len())
        .expect("no resize was done")
        .0;

    Ok(Resized {
        resized: resized,
        smallest_image: images.swap_remove(smallest_idx),
    })
}

