use serde::{Deserialize, Serialize};

use crate::model::{Identifier, Properties};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoMeta {
    pub id: Identifier,
    pub images: Vec<ImageMeta>,
    pub properties: PropertiesMeta,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageMeta {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub image_id: String,
    pub extension: String,
}

pub type PropertiesMeta = Properties;

