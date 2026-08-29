use std::collections::HashSet;

use anyhow::Context as _;

use crate::model::{ControlledField, PhotoMeta};

#[derive(thiserror::Error, Debug)]
pub enum Rejections {
    #[error("The touched property are being controlled by the Iris - the consumer cannot change this property: {:?}", .0)]
    ControlledProperty(HashSet<&'static ControlledField>),
}

#[derive(thiserror::Error, Debug)]
pub enum ChangeValidationError {
    #[error("The new PhotoMeta cannot be accepted for update:\n- {}", .0.iter().map(|e| format!("- {e}\n")).collect::<String>())]
    Rejected(Vec<Rejections>),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub fn try_apply_update(
    original: &PhotoMeta,
    mut new: PhotoMeta,
) -> Result<PhotoMeta, ChangeValidationError> {
    // This field is omitted when serializing into the PhotoScheme
    // TODO: Make information loss detectable in type level by creating distinct type
    new.original = original.original.clone();
    new.properties.orientation = original.properties.orientation.clone();

    let changed_paths = json_patch::diff(
        &serde_json::to_value(original)
            .context("Converting PhotoMeta back to the JSON for validation")?,
        &serde_json::to_value(&new)
            .context("Converting PhotoMeta back to the JSON for validation")?,
    );

    let mut controlled_property = HashSet::new();

    for patch in &*changed_paths {
        if let Some(disallowed) = PhotoMeta::controlled_field(patch.path()) {
            controlled_property.insert(disallowed);
        }
    }

    if !controlled_property.is_empty() {
        return Err(ChangeValidationError::Rejected(vec![
            Rejections::ControlledProperty(controlled_property),
        ]));
    }

    Ok(new)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, hash::RandomState};

    use crate::{
        assert_let,
        model::{
            ImageMeta, LocalIdentifier, PhotoMeta, PhotoOrigin,
            service::photo::{ChangeValidationError, Rejections, try_apply_update},
        },
    };

    fn photo_meta() -> PhotoMeta {
        serde_json::from_str(include_str!("./test-photo-meta.json")).unwrap()
    }

    fn image_meta() -> ImageMeta {
        ImageMeta {
            width: 1024,
            height: 1024,
            mime: "image/png".to_string(),
            extension: "png".to_string(),
        }
    }

    #[test]
    pub fn the_same_object_is_accepted() {
        let original = photo_meta();

        std::assert_matches!(try_apply_update(&original, original), Ok(_));
    }

    #[test]
    pub fn non_critical_property_change_is_accepted() {
        let original = photo_meta();
        let mut changed = original.clone();

        changed.tags.push("foo/bar".parse().unwrap());
        changed.properties.machine = "New machine".to_string();
        changed.properties.gps_lat_lng = None;
        changed.properties.lens = None;
        changed.properties.f_number = None;
        changed.properties.shutter_speed = None;
        changed.properties.shutter_speed_controlled = None;
        changed.properties.iso = None;
        changed.properties.focal = None;
        changed.properties.orientation = None;
        changed.shot_time = chrono::DateTime::default();
        changed.representative_rgb = [255, 255, 255];

        std::assert_matches!(try_apply_update(&original, changed), Ok(_));
    }

    #[test]
    pub fn critical_property_change_is_not_accepted() {
        let original = photo_meta();
        let mut changed = original.clone();

        let mut changing_paths = Vec::new();

        changing_paths.push("/origin");
        changed.origin = PhotoOrigin::Local(LocalIdentifier("202501-01K".parse().unwrap()));

        changing_paths.push("/original");
        changed.original = Some(image_meta());

        changing_paths.push("/images");
        changed.images.insert("test".to_string(), image_meta());

        changing_paths.push("/original_sha256");
        changed.original_sha256 = "foobar".to_string();

        let validation = try_apply_update(&original, changed);

        assert_let! {
            Err(ChangeValidationError::Rejected(rejections)) = &validation
        };
        assert_let! {
            [Rejections::ControlledProperty(rejected_pointers)] = rejections.as_slice()
        };

        let expected_rejection = HashSet::<_, RandomState>::from_iter(changing_paths);
        let actual_rejection = HashSet::<_, RandomState>::from_iter(
            rejected_pointers.iter().map(|ptr| ptr.pointer.as_str()),
        );

        let missed = expected_rejection
            .difference(&actual_rejection)
            .collect::<Vec<_>>();

        std::assert_matches!(missed.as_slice(), []);
    }
}
