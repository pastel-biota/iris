use std::sync::Arc;

use crate::{Context, repository::io::ScopedPath};

/// Migrate Iris data to be usable on the next version of data schema.
/// This help only contains migrations available in this version of Iris.
#[derive(Clone, Debug, clap::Subcommand)]
pub enum MigrationOptions {
    #[doc(hidden)]
    #[clap(hide = true)]
    /// Migrate JSON-based fast reference into the SQLite.
    V20260723,
}

pub async fn migrate(_ctx: Arc<Context>, _ingest_scope: ScopedPath, option: MigrationOptions) -> anyhow::Result<()> {
    match option {
        MigrationOptions::V20260723 => migrate_v20260723_sqlite().await,
    }
}

async fn migrate_v20260723_sqlite() -> anyhow::Result<()> {
    indoc::printdoc! {"

    V20260723 - Migrate JSON-based fast reference into the SQLite.
    This migration will affect to these information:
      - Local photo index.
      - Remote photo index.
        (The remote photo index can only be migrated through this utility)

    **This migration is no longer available. Use the binary built before v3.1.4.**
    "}

    anyhow::bail!("This migration is no longer available")
}



