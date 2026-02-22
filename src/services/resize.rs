use std::io::Cursor;

use image::{ImageFormat, ImageReader, ImageResult, imageops::FilterType};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator};

pub struct ResizeTargets {
    pub w: u32,
    pub h: u32,
    pub id: &'static str,
    pub ext: ImageFormat,
}

pub const RESIZE_TARGETS: [ResizeTargets; 4] = [
    ResizeTargets { w: 128, h: 128, id: "icon", ext: ImageFormat::WebP },
    ResizeTargets { w: 480, h: 480, id: "thumbnail", ext: ImageFormat::WebP },
    ResizeTargets { w: 1920, h: 1080, id: "main", ext: ImageFormat::WebP },
    ResizeTargets { w: 2560, h: 1440, id: "highres", ext: ImageFormat::Png },
];

pub struct Resized {
    pub resized: Vec<(&'static ResizeTargets, Vec<u8>)>,
}

pub async fn resize_images(original: &[u8]) -> anyhow::Result<Resized> {
    tracing::info!("Decoding");
    let original = ImageReader::new(Cursor::new(original))
        .with_guessed_format()?
        .decode()?;
    tracing::info!("Finished decoding. Strarting resize");

    let resized = tokio::task::spawn_blocking(move || {
        RESIZE_TARGETS
            .par_iter()
            .map(|target| {
                let mut byte = Cursor::new(Vec::new());
                tracing::debug!("Resizing: {}", target.id);
                original
                    .resize(target.w, target.h, FilterType::Gaussian)
                    .write_to(&mut byte, target.ext)?;
                tracing::debug!("Resized!: {}", target.id);
                ImageResult::Ok((target, byte.into_inner()))
            })
            .collect::<Result<Vec<_>, _>>()
    }).await??;

    Ok(Resized {
        resized
    })
}

