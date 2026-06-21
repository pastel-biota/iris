use std::collections::HashMap;

use anyhow::{Context as _, bail};

use crate::{model::{Identifier, ImageMeta, PhotoOrigin, PhotoReference}, repository::io::ScopedPath};

#[derive(Debug)]
pub struct DateImageIndex {
    path: ScopedPath,
    content: Option<DateImageIndexEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
enum DateImageIndexEntry {
    V1(v1::ImageIndexEntry),
}

impl Default for DateImageIndexEntry {
    fn default() -> Self {
        DateImageIndexEntry::V1(Default::default())
    }
}

mod v1 {
    use std::collections::HashMap;
    use chrono::{DateTime, FixedOffset};

    use crate::model::{PhotoOrigin, PhotoReference};

    #[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
    pub(super) struct ImageIndexEntry {
        pub total_count: u32,
        pub pics: HashMap<String, HashMap<String, Vec<IndexElement>>>,
    }
    
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub(super) struct IndexElement {
        pub origin: PhotoOrigin,
        pub shot_time: DateTime<FixedOffset>,
    }

    impl ImageIndexEntry {
        pub(super) fn registered_months(&self) -> anyhow::Result<Vec<(i32, u32)>> {
            let mut months = self
                .pics
                .iter()
                .flat_map(|(year, months)| {
                    months
                        .keys()
                        .map(|month| (year.parse::<i32>().unwrap(), month.parse::<u32>().unwrap()))
                })
                .collect::<Vec<_>>();
    
            months.sort();
            months.reverse();
    
            Ok(months)
        }
    
        pub(super) fn iterate_photo_in_month(&mut self, year: i32, month: u32) -> &mut Vec<IndexElement> {
            self.pics
                .entry(year.to_string())
                .or_default()
                .entry(month.to_string())
                .or_default()
        }
    }

    impl From<PhotoReference> for IndexElement {
        fn from(value: PhotoReference) -> Self {
            Self {
                origin: value.origin.clone(),
                shot_time: value.shot_time.clone(),
            }
        }
    }
}


impl DateImageIndex {
    pub fn new(path: &ScopedPath) -> DateImageIndex {
        DateImageIndex {
            path: path.clone(),
            content: None,
        }
    }

    pub fn upsert(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        let DateImageIndexEntry::V1(index) = self.load()?;

        let month_pics = index
            .pics
            .entry(photo.id().year.to_string())
            .or_default()
            .entry(photo.id().month.to_string())
            .or_default();

        if let Some(stored_photo) = month_pics.iter_mut().find(|stored_photo| stored_photo.origin.id() == photo.id()) {
            *stored_photo = photo.clone().into();
        } else {
            // figure out where to insert. The array should be sorted by shot_time's asc
            let month_index = month_pics
                .iter()
                .position(|stored_path| stored_path.shot_time > photo.shot_time)
                .unwrap_or(month_pics.len());
            index.total_count += 1;
            month_pics.insert(month_index, photo.clone().into());
        }

        self.save()?;

        Ok(())
    }

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        let DateImageIndexEntry::V1(index) = self.load()?;
        Ok(index.total_count)
    }

    pub fn list_first_n_images(&mut self, size: usize) -> anyhow::Result<Vec<&PhotoOrigin>> {
        let DateImageIndexEntry::V1(index) = self.load()?;

        let images = index
            .registered_months()?
            .into_iter()
            .flat_map(|(year, month)| {
                index.pics[&year.to_string()][&month.to_string()]
                    .iter()
                    .rev()
            });

        Ok(images.take(size).map(|index| &index.origin).collect())
    }

    pub fn list_images_beginning_from_photo(
        &mut self,
        ident: &Identifier,
        size: usize,
    ) -> anyhow::Result<Vec<&PhotoOrigin>> {
        let DateImageIndexEntry::V1(index) = self.load()?;

        let photos = index
            .pics
            .get(&ident.year.to_string())
            .and_then(|year| year.get(&ident.month.to_string()))
            .with_context(|| {
                format!("The month {}/{} is not registered", ident.year, ident.month)
            })?;

        let month_image = photos
            .iter()
            .rev()
            .skip_while(|photo| photo.origin.id() != ident)
            .skip(1);

        let following_images = index
            .registered_months()?
            .into_iter()
            .skip_while(|month| *month >= (ident.year, ident.month))
            .flat_map(|(year, month)| {
                index.pics[&year.to_string()][&month.to_string()]
                    .iter()
                    .rev()
            });

        let images = month_image.chain(following_images);

        Ok(images.take(size).map(|index| &index.origin).collect())
    }

    pub fn delete_photo(&mut self, photo_id: &Identifier) -> anyhow::Result<PhotoOrigin> {
        let DateImageIndexEntry::V1(index) = self.load()?;

        let mut result: Option<anyhow::Result<PhotoOrigin>> = None;
        index
            .iterate_photo_in_month(photo_id.year, photo_id.month)
            .retain(|photo| {
                // element returning false is removed

                if result.is_some() {
                    return true;
                }

                if photo.origin.id() != photo_id {
                    return true;
                }

                if photo.origin.federated() {
                    result = Some(Err(anyhow::anyhow!("The photo was found but it is federated. You need to remove it from the authority")));
                    return true;
                }

                result = Some(Ok(photo.origin.clone()));
                false
            });

        result.unwrap_or(Err(anyhow::anyhow!("The photo was not found")))
    }

    fn load(&mut self) -> anyhow::Result<&mut DateImageIndexEntry> {
        // Fusing these two fns is really hard somehow
        self._load()?;
        Ok(self.content.as_mut().expect("Just initialized"))
    }

    fn _load(&mut self) -> anyhow::Result<()> {
        if self.content.is_some() {
            return Ok(());
        }

        if !self.path.exists() {
            self.content = Some(self.init()?);
            return Ok(());
        }

        let bytes = self
            .path
            .read_binary()
            .context("Failed to open a file for all image index")?;

        self.content = serde_json::from_slice(&bytes)
            .context("The all image index contains invalid content")?;

        Ok(())
    }

    fn save(&mut self) -> anyhow::Result<()> {
        let bytes = {
            let entry = self.load()?;
            serde_json::to_vec_pretty(entry)
                .context("Failed to serialize the all image index")?
        };

        self.path
            .write(bytes)
            .context("Failed to write the all image index")?;

        Ok(())
    }

    fn init(&mut self) -> anyhow::Result<DateImageIndexEntry> {
        let value = DateImageIndexEntry::default();
        let bytes = serde_json::to_vec_pretty(&value)
            .context("Failed to serialize an empty entry for all image index")?;

        self.path
            .write(bytes)
            .context("Failed to create a file for all image index")?;

        Ok(value)
    }
}
