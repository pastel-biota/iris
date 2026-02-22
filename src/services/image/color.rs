use image::{DynamicImage, GenericImageView, Pixel, Rgb, Rgba};

pub fn average_color(image: &DynamicImage) -> Rgb<u8> {
    let w = image.width();
    let h = image.width();
    image.fast_blur(1024.0)
        .get_pixel(w / 2, h / 2)
        .to_rgb()
}

