use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Context as _;

use crate::{infra::photo_index::PhotoReference, model::PhotoMeta};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AllImageIndex {
    path: PathBuf,
    content: Option<AllImageIndexEntry>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct AllImageIndexEntry {
    total_count: u32,
    pics: HashMap<i32, HashMap<u32, Vec<PhotoReference>>>,
}

impl AllImageIndex {
    pub fn new(path: &Path) -> AllImageIndex {
        AllImageIndex {
            path: path.to_path_buf(),
            content: None,
        }
    }

    pub fn add(&mut self, photo: &PhotoMeta) -> anyhow::Result<()> {
        let index = self.load()?;

        let month_pics = index
            .pics
            .entry(photo.id.year)
            .or_insert(HashMap::new())
            .entry(photo.id.month)
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

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        let index = self.load()?;
        Ok(index.total_count)
    }

    pub fn list_images(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        let index = self.load()?;

        let mut cursor: usize = 0;
        let mut years = index.pics.iter().collect::<Vec<_>>();
        years.sort_by_key(|(year, _)| -**year);

        let mut refs = Vec::with_capacity(limit);
        for (_, months) in years {
            let mut months = months.iter().collect::<Vec<_>>();
            months.sort_by_key(|(month, _)| **month);
            months.reverse();

            for (_, image) in months {
                let skipping_elems = if cursor < offset {
                    (offset - cursor).min(image.len())
                } else {
                    0
                };

                let required_elems = limit - refs.len();

                let adding_refs = image
                    .iter()
                    .rev()
                    .skip(skipping_elems)
                    .take(required_elems)
                    .cloned()
                    .collect::<Vec<_>>();
                cursor += skipping_elems + adding_refs.len();

                refs.extend(adding_refs);
            }
        }

        Ok(refs)
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
