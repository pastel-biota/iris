use std::collections::HashMap;

use anyhow::Context as _;

use crate::{model::{Identifier, ImageMeta, LocalIdentifier, PhotoReference}, repository::{io::ScopedPath, photo_index::{PhotoIndexProvider}}};

pub struct ReferenceIndex {
    path: ScopedPath,
    content: Option<IndexEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
pub enum IndexEntry {
    V1(v1::IndexEntry),
}

impl Default for IndexEntry {
    fn default() -> Self {
        IndexEntry::V1(Default::default())
    }
}

mod v1 {
    use std::collections::HashMap;
    use chrono::{DateTime, FixedOffset};
    use crate::model::{self, Identifier};

    #[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
    pub(super) struct IndexEntry {
        pub total_count: u32,
        pub pics: HashMap<Identifier, PhotoReference>,
    }

    macro_rules! symmetrical_from_into {
        (
            #[$($attr:meta)+]
            $pub:vis struct $ident:ident (= $equiv:path) {
                $($fpub:vis $fident:ident : $ty:ty $(|$fn_ident:ident| -> $rty:ty, $rty2:ty => $expr:expr)* ,)+
            }
        ) => {
            #[$($attr)+]
            $pub struct $ident {
                $($fpub $fident : $ty ,)+
            }

            impl From<$equiv> for $ident {
                fn from(equiv: $equiv) -> $ident {
                    $ident {
                        $($fident : {
                            let ret = equiv . $fident;
                            $(
                                let func = |$fn_ident: $rty2| -> $rty { type Rty = $rty; $expr };
                                let ret : $rty = func(ret);
                            )*
                            ret
                        },)+
                    }
                }
            }

            impl Into<$equiv> for $ident {
                fn into(self) -> $equiv {
                    $equiv {
                        $($fident : {
                            let ret = self . $fident;
                            $(
                                let func = |$fn_ident: $rty| -> $rty2 { type Rty = $rty2; $expr };
                                let ret : $rty2 = func(ret);
                            )*
                            ret
                        },)+
                    }
                }
            }
        };
    }

    symmetrical_from_into! {
        #[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
        pub struct PhotoReference (= model::PhotoReference) {
            pub origin: model::PhotoOrigin,
            pub year: i32,
            pub month: u32,
            pub hash: String,
            pub images: HashMap<String, ImageMeta> |x| -> 
                HashMap<String, ImageMeta>,
                HashMap<String, model::ImageMeta>
            => x.into_iter().map(|(k, v)| (k, v.into())).collect::<Rty>(),
            pub shot_time: DateTime<FixedOffset>,
            pub representative_rgb: [u8; 3],
        }
    }

    symmetrical_from_into! {
        #[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
        pub struct ImageMeta (= model::ImageMeta) {
            pub width: u32,
            pub height: u32,
            pub extension: String,
            pub mime: String,
        }
    }
}


impl PhotoIndexProvider for ReferenceIndex {
    const INDEX_NAME: &'static str = "sha256 index";
    type Entry = IndexEntry;

    fn upsert(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let replaced = index
            .pics
            .insert(photo.id().clone(), photo.clone().into());

        if replaced.is_none() {
            index.total_count += 1;
        }

        self.save()?;

        Ok(())
    }

    fn total_count(&mut self) -> anyhow::Result<u32> {
        let IndexEntry::V1(index) = self.load_mut()?;
        Ok(index.total_count)
    }
}

impl ReferenceIndex {
    pub fn new(path: &ScopedPath) -> ReferenceIndex {
        ReferenceIndex {
            path: path.clone(),
            content: None,
        }
    }

    pub fn get_photo(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(index.pics.get(id).cloned().map(|refs| refs.into()))
    }

    pub fn bulk_load_photo_map<'a>(&mut self, id: impl IntoIterator<Item = &'a Identifier>) -> anyhow::Result<HashMap<Identifier, PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(id.into_iter()
            .filter_map(|id| index.pics.get(&*id).map(|photo| ((*id).clone(), photo.clone().into())))
            .collect())
    }

    pub fn bulk_load_photo<'a>(&mut self, id: impl IntoIterator<Item = &'a Identifier>) -> anyhow::Result<impl Iterator<Item = Option<PhotoReference>>> {

        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(id.into_iter().map(|id| index.pics.get(&*id).map(|photo| photo.clone().into())))
    }

    pub fn add_new_image(
        &mut self,
        photo_id: &LocalIdentifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let photo = index
            .pics
            .get_mut(&photo_id.0)
            .context("The image was not found")?;

        photo
            .images
            .insert(image_id.to_string(), image.clone().into());

        self.save()?;

        Ok(())
    }

    pub fn delete_photo(
        &mut self,
        photo_id: &Identifier,
    ) -> anyhow::Result<PhotoReference> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let photo = index
            .pics
            .remove(&photo_id)
            .context("The image was not found")?;

        Ok(photo.into())
    }

    fn load_mut(&mut self) -> anyhow::Result<&mut IndexEntry> {
        if self.content.is_none() {
            let path = self.path.clone();
            let entry = self.load_to_file(&path)?;
            return Ok(self.content.insert(entry));
        }

        Ok(self.content.as_mut().unwrap())
    }

    fn save(&mut self) -> anyhow::Result<()> {
        let bytes = {
            let entry = self.load_mut()?;
            serde_json::to_vec_pretty(entry)
                .context("Failed to serialize the sha256 index")?
        };

        self.path
            .write(bytes)
            .context("Failed to write the sha256 index")?;

        Ok(())
    }
}
