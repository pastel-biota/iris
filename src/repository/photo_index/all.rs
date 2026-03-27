use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail};

use crate::{
    repository::photo_index::PhotoReference,
    model::{Identifier, ImageMeta, PhotoMeta},
};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AllImageIndex {
    path: PathBuf,
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
}

impl AllImageIndex {
    pub fn new(path: &Path) -> AllImageIndex {
        AllImageIndex {
            path: path.to_path_buf(),
            content: None,
        }
    }

    pub fn add(&mut self, photo: &PhotoMeta) -> anyhow::Result<()> {
        let AllImageIndexEntry::V1(index) = self.load()?;

        let month_pics = index
            .pics
            .entry(photo.id.year.to_string())
            .or_insert(HashMap::new())
            .entry(photo.id.month.to_string())
            .or_insert(vec![]);

        if month_pics
            .iter()
            .any(|stored_photo| stored_photo.id == photo.id)
        {
            anyhow::bail!("There already is an identifier with that photo")
        }

        index.total_count += 1;

        // figure out where to insert. The array should be sorted by shot_time's asc
        let index = month_pics
            .iter()
            .position(|stored_path| stored_path.shot_time > photo.shot_time)
            .unwrap_or(month_pics.len());
        month_pics.insert(index, photo.clone().into());

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
            .and_then(|month| month.iter_mut().find(|photo| &photo.id == photo_id))
        else {
            bail!("Image was not found")
        };

        photo
            .images
            .insert(image_id.to_string(), image.clone().into());

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
            .skip_while(|photo| &photo.id != ident)
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
            dbg!(&self.path);
            self.content = Some(self.init()?);
            return Ok(());
        }

        let file = File::open(&self.path).context("Failed to open a file for all image index")?;

        self.content = serde_json::from_reader(file)
            .context("The all image index contains invalid content")?;

        Ok(())
    }

    fn save(&mut self) -> anyhow::Result<()> {
        let mut file =
            File::create(&self.path).context("Failed to create a file for all image index")?;

        serde_json::to_writer_pretty(&mut file, &self.load()?)
            .context("Failed to write a empty entry for all image index")?;

        Ok(())
    }

    fn init(&mut self) -> anyhow::Result<AllImageIndexEntry> {
        let mut file =
            File::create(&self.path).context("Failed to create a file for all image index")?;

        let value = AllImageIndexEntry::default();
        serde_json::to_writer_pretty(&mut file, &value)
            .context("Failed to write a empty entry for all image index")?;

        Ok(value)
    }
}
