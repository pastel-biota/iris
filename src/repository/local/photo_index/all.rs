use std::collections::HashMap;

use anyhow::{Context as _, bail};

use crate::{model::{Identifier, ImageMeta, PhotoMeta, PhotoReference}, repository::io::ScopedPath};

#[derive(Debug)]
pub struct AllImageIndex {
    path: ScopedPath,
    content: Option<AllImageIndexEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
enum AllImageIndexEntry {
    V1(AllImageIndexEntryV1),
}

impl Default for AllImageIndexEntry {
    fn default() -> Self {
        AllImageIndexEntry::V1(Default::default())
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct AllImageIndexEntryV1 {
    total_count: u32,
    pics: HashMap<String, HashMap<String, Vec<PhotoReference>>>,
}

impl AllImageIndexEntryV1 {
    fn registered_months(&self) -> anyhow::Result<Vec<(i32, u32)>> {
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

    fn iterate_photo_in_month(&mut self, year: i32, month: u32) -> &mut Vec<PhotoReference> {
        self.pics
            .entry(year.to_string())
            .or_default()
            .entry(month.to_string())
            .or_default()
    }
}

impl AllImageIndex {
    pub fn new(path: &ScopedPath) -> AllImageIndex {
        AllImageIndex {
            path: path.clone(),
            content: None,
        }
    }

    pub fn upsert(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        let AllImageIndexEntry::V1(index) = self.load()?;

        let month_pics = index
            .pics
            .entry(photo.id().year.to_string())
            .or_default()
            .entry(photo.id().month.to_string())
            .or_default();

        if let Some(stored_photo) = month_pics.iter_mut().find(|stored_photo| stored_photo.id() == photo.id()) {
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

    pub fn add_new_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<()> {
        let AllImageIndexEntry::V1(index) = self.load()?;

        let Some(photo) = index
            .pics
            .get_mut(&photo_id.year.to_string())
            .and_then(|year| year.get_mut(&photo_id.month.to_string()))
            .and_then(|month| month.iter_mut().find(|photo| photo.id() == photo_id))
        else {
            bail!("Image was not found")
        };

        photo
            .images
            .insert(image_id.to_string(), image.clone());

        self.save()?;

        Ok(())
    }

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        let AllImageIndexEntry::V1(index) = self.load()?;
        Ok(index.total_count)
    }

    pub fn list_first_n_images(&mut self, size: usize) -> anyhow::Result<Vec<PhotoReference>> {
        let AllImageIndexEntry::V1(index) = self.load()?;

        let images = index
            .registered_months()?
            .into_iter()
            .flat_map(|(year, month)| {
                index.pics[&year.to_string()][&month.to_string()]
                    .iter()
                    .rev()
            });

        Ok(images.take(size).cloned().collect())
    }

    pub fn list_images_beginning_from_photo(
        &mut self,
        ident: &Identifier,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        let AllImageIndexEntry::V1(index) = self.load()?;

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
            .skip_while(|photo| photo.id() != ident)
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

        Ok(images.take(size).cloned().collect())
    }

    pub fn delete_photo(&mut self, photo_id: &Identifier) -> anyhow::Result<PhotoReference> {
        let AllImageIndexEntry::V1(index) = self.load()?;

        let mut result: Option<anyhow::Result<PhotoReference>> = None;
        index
            .iterate_photo_in_month(photo_id.year, photo_id.month)
            .retain(|photo| {
                // element returning false is removed

                if result.is_some() {
                    return true;
                }

                if photo.id() != photo_id {
                    return true;
                }

                if photo.origin.federated() {
                    result = Some(Err(anyhow::anyhow!("The photo was found but it is federated. You need to remove it from the authority")));
                    return true;
                }

                result = Some(Ok(photo.clone()));
                false
            });

        result.unwrap_or(Err(anyhow::anyhow!("The photo was not found")))
    }

    fn load(&mut self) -> anyhow::Result<&mut AllImageIndexEntry> {
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

    fn init(&mut self) -> anyhow::Result<AllImageIndexEntry> {
        let value = AllImageIndexEntry::default();
        let bytes = serde_json::to_vec_pretty(&value)
            .context("Failed to serialize an empty entry for all image index")?;

        self.path
            .write(bytes)
            .context("Failed to create a file for all image index")?;

        Ok(value)
    }
}
