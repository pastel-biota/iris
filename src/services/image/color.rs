use image::{DynamicImage, GenericImageView, Pixel, Rgb, Rgba};

pub fn average_color(image: &DynamicImage) -> Rgb<u8> {
    let w = image.width();
    let h = image.height();
    let x_dead_w = image.width() / 15;
    let y_dead_h = image.height() / 15;

    let left_dead = 0..x_dead_w;
    let right_dead = (w-x_dead_w)..w;
    let top_dead = 0..y_dead_h;
    let bottom_dead = (h-y_dead_h)..h;

    let averaged_color = image.pixels()
        .filter(|(x, y, pixel)| {
            !left_dead.contains(&x) &&
            !right_dead.contains(&x) &&
            !top_dead.contains(&y) &&
            !bottom_dead.contains(&y) &&
            !too_dark(pixel)
        })
        .map(|(_, _, pixel)| pixel.to_rgb().0)
        .map(|[r, g, b]| (r as f32, g as f32, b as f32, 1))
        .reduce(|(ar, ag, ab, c), (r, g, b, _)| {
            (
                ar + r,
                ag + g,
                ab + b,
                c + 1,
            )
        });

    if let Some((r, g, b, pixels)) = averaged_color {
        Rgb::from([
            (r / pixels as f32) as u8,
            (g / pixels as f32) as u8,
            (b / pixels as f32) as u8,
        ])
    } else {
        Rgb::from([60, 60, 60])
    }

}

fn too_dark(rgba: &Rgba<u8>) -> bool {
    let [r, g, b, _] = rgba.0;

    if [r, g, b].into_iter().all(|v| v < 64) {
        return true;
    }

    let diff = [
        r.abs_diff(g),
        g.abs_diff(b),
        r.abs_diff(b),
    ].into_iter().max().unwrap();

    diff < 16
}

