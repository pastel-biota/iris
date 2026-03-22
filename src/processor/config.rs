use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ImageProcessConfig {
    pub sizes: HashMap<String, ResizeTargets>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResizeTargets {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    PNG,
    WEBP,
    JPEG,
}

