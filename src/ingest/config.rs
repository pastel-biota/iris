use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct IngestConfig {
    pub dir: PathBuf,
    pub listen: String,
    pub cors_origin: Vec<String>,
}
