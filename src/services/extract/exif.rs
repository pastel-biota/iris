use std::io::Cursor;

use anyhow::{Context, bail};

use chrono::{DateTime, FixedOffset};
use exif::{Exif, In, Tag, Value};

use crate::model::{Orientation, Properties, Rational, Rotation};

pub struct ExifPayload {
    pub props: Properties,
    pub shot_time: DateTime<FixedOffset>,
}

pub async fn read_exif(bytes: &[u8]) -> anyhow::Result<ExifPayload> {
    let mut bytes = Cursor::new(bytes);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bytes)?;
    exif_to_image_property(&ExifData(exif))
}

struct ExifData(Exif);

impl ExifData {
    pub fn get_ascii(&self, tag: Tag) -> Option<&str> {
        let Value::Ascii(ref exif_ascii) = self.0.get_field(tag, In::PRIMARY)?.value else {
            return None;
        };

        let array = exif_ascii.first()?;
        str::from_utf8(array.as_slice()).ok()
    }

    pub fn get_indexed(&self, tag: Tag) -> Option<u32> {
        self.0.get_field(tag, In::PRIMARY)?.value.get_uint(0)
    }

    pub fn get_rational(&self, tag: Tag) -> anyhow::Result<Option<Rational>> {
        let Some(field) = self.0.get_field(tag, In::PRIMARY) else {
            return Ok(None);
        };

        match field.value {
            Value::Rational(ref exif_rational) => {
                let Some(exif_rational) = exif_rational.first() else {
                    return Ok(None);
                };
                Ok(Some(Rational(
                    exif_rational
                        .denom
                        .try_into()
                        .context("Rational value from EXIF is out of range")?,
                    exif_rational
                        .num
                        .try_into()
                        .context("Rational value from EXIF is out of range")?,
                )))
            }
            Value::SRational(ref exif_rational) => {
                let Some(exif_rational) = exif_rational.first() else {
                    return Ok(None);
                };
                Ok(Some(Rational(exif_rational.denom, exif_rational.num)))
            }
            _ => Ok(None),
        }
    }

    #[allow(unused)]
    pub fn display_report(&self) {
        for field in self.0.fields() {
            println!("\x1b[38;5;5m{}\x1b[m [{}]", field.tag, field.tag.1);
            println!("   {}", field.display_value());
            println!("   \x1b[38;5;245m{:?}\x1b[m", field.value);
        }
    }
}

fn exif_to_image_property(exif: &ExifData) -> anyhow::Result<ExifPayload> {
    let gps_lat = gps_degree(exif, Tag::GPSLatitude, Tag::GPSLatitudeRef, ("S", "N"))?;
    let gps_long = gps_degree(exif, Tag::GPSLongitude, Tag::GPSLongitudeRef, ("W", "E"))?;
    let gps_lat_lng = gps_lat.and_then(|lat| gps_long.map(|long| (lat, long)));

    let f_number = exif
        .get_rational(Tag::FNumber)?
        .context("FNumber is not available")?
        .to_f64();
    let shutter_speed = exif
        .get_rational(Tag::ExposureTime)?
        .context("ShutterSpeed is not available")?
        .normalize_to_one();
    let shutter_speed_controlled = exif
        .get_indexed(Tag::ExposureMode)
        .context("ExposureMode is not available")?
        .eq(&1);
    let iso = exif
        .get_indexed(Tag::PhotographicSensitivity)
        .context("ISO value is not available")?
        .into();
    let focal = exif
        .get_rational(Tag::FocalLength)?
        .context("Focus value is not available")?
        .to_f64();

    Ok(ExifPayload {
        shot_time: time_value(exif, Tag::DateTime, Tag::OffsetTime)?,
        props: Properties {
            machine: machinery(exif, Tag::Make, Tag::Model)?,
            lens: Some(machinery(exif, Tag::LensMake, Tag::LensModel)?),
            orientation: Some(get_orientation(exif)?),
            gps_lat_lng,
            f_number: Some(f_number),
            shutter_speed: Some(shutter_speed),
            shutter_speed_controlled: Some(shutter_speed_controlled),
            iso: Some(iso),
            focal: Some(focal),
        },
    })
}

/// Falls back to when the camera didn't write `OffsetTime` (common when
/// the timezone setting isn't configured in-camera).
const FALLBACK_OFFSET: &str = "+09:00";

fn time_value(
    exif: &ExifData,
    date_time: Tag,
    offset_time: Tag,
) -> anyhow::Result<chrono::DateTime<FixedOffset>> {
    // YYYY:MM:DD hh:mm:ss
    let timestamp = exif
        .get_ascii(date_time)
        .context("DateTime is not available")?;
    let timezone = exif
        .get_ascii(offset_time)
        .unwrap_or(FALLBACK_OFFSET);

    chrono::DateTime::parse_from_str(&format!("{timestamp} {timezone}"), "%Y:%m:%d %H:%M:%S %:z")
        .context("DateTime was malformed")
}

fn get_orientation(exif: &ExifData) -> anyhow::Result<Orientation> {
    let orientation = exif
        .get_indexed(Tag::Orientation)
        .context("Orientation is not available")?;
    Ok(match orientation {
        1 => Orientation {
            rotation: Rotation::Upright,
            flip: false,
        },
        2 => Orientation {
            rotation: Rotation::Upright,
            flip: true,
        },
        3 => Orientation {
            rotation: Rotation::UpsideDown,
            flip: false,
        },
        4 => Orientation {
            rotation: Rotation::UpsideDown,
            flip: true,
        },
        5 => Orientation {
            rotation: Rotation::CounterClockwise,
            flip: true,
        },
        6 => Orientation {
            rotation: Rotation::Clockwise,
            flip: false,
        },
        7 => Orientation {
            rotation: Rotation::Clockwise,
            flip: true,
        },
        8 => Orientation {
            rotation: Rotation::CounterClockwise,
            flip: false,
        },
        _ => bail!("Orientation value was not in expected range"),
    })
}

fn machinery(exif: &ExifData, vendor: Tag, model: Tag) -> anyhow::Result<String> {
    let vendor = exif
        .get_ascii(vendor)
        .map(|vendor| vendor.trim().to_string());
    let model = exif
        .get_ascii(model)
        .context("Camera's model is not available")?
        .trim()
        .to_string();

    if let Some(vendor) = vendor && !model.contains(&vendor) {
        Ok(format!("{vendor} {model}"))
    } else {
        Ok(model)
    }
}

fn gps_degree(
    exif: &ExifData,
    value: Tag,
    refs: Tag,
    (minus, plus): (&str, &str),
) -> anyhow::Result<Option<f64>> {
    let Some(field) = exif.0.get_field(value, In::PRIMARY) else {
        return Ok(None);
    };

    let Value::Rational(components) = &field.value else {
        bail!("GPS Latitude was not in the expected form");
    };

    let [deg, min, sec] = components.as_slice() else {
        bail!("GPS Latitude was not in the expected form");
    };

    let Some(refs) = exif.get_ascii(refs) else {
        return Ok(None);
    };

    let sign = if refs == minus {
        -1.0
    } else if refs == plus {
        1.0
    } else {
        bail!("The reference was not provided in the expected form");
    };

    let long_abs = deg.to_f64() + min.to_f64() / 60.0 + sec.to_f64() / 3600.0;

    Ok(Some(long_abs * sign))
}
