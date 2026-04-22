// This module is a boundary between user-provided configuration and the
// `ScopedPath`-based storage layer. Raw `PathBuf` is needed here for TOML
// deserialization; it gets wrapped in `ScopedPath` at the point of use.
#![allow(clippy::disallowed_types)]

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct IngestConfig {
    pub dir: PathBuf,
    pub listen: String,
    pub cors_origin: Vec<String>,
}
