use std::{collections::HashMap, io::{Read as _, Write}, sync::Arc};

use anyhow::Context as _;
use tracing::Instrument;

use crate::{Context, model::EntityName, repository::{io::ScopedPath, photo_index::reference::ReferenceIndex}};

#[derive(Clone, Debug, clap::Subcommand)]
pub enum MigrationOptions {
    /// Migrate JSON-based fast reference into the SQLite.
    V20260723,
}

pub async fn migrate(ctx: Arc<Context>, ingest_scope: ScopedPath, option: MigrationOptions) -> anyhow::Result<()> {
    match option {
        MigrationOptions::V20260723 => migrate_v20260723_sqlite(ctx, ingest_scope).await,
    }
}

async fn migrate_v20260723_sqlite(ctx: Arc<Context>, ingest_scope: ScopedPath) -> anyhow::Result<()> {
    indoc::printdoc! {"

    V20260723 - Migrate JSON-based fast reference into the SQLite.
    This migration will affect to these information:
      - Local photo index.
      - Remote photo index.
        (The remote photo index can only be migrated through this utility)

    This version of Iris is available for this migration.

      * This migration is indestructive. No file will be removed.
      * This migration will supersede some files.

    This migration is interactive.

       1. The migration will execute the data retrieval first.
       2. After reviewing the data is actually migrated.
    "}

    indoc::printdoc! {"
    
    1. Retrieve data
    -----------------

    Iris will retrieve the data first. If you have Iris running, you should stop
    or make sure no job is running. Do you want to continue?: [yN] "};
    std::io::stdout().flush()?;

    if !confirm_from_user()? {
        println!("Aborted.");
        return Ok(());
    }

    println!(":: Preparing JSON-based key...");
    let mut json_registry = ReferenceIndex::new(&ingest_scope.join("_pics.json"));

    let total_count = json_registry.total_count()?;
    println!(":: {} photos are found.", total_count);
    println!(":: Will read the file first to see the photos...");

    let mut photos_by_federation = HashMap::<Option<EntityName>, usize>::new();

    json_registry.dangerously_read_photo()?.for_each(|photo| {
        let count = photos_by_federation
            .entry(photo.origin.federator().cloned())
            .or_insert(0);
        *count += 1;
    });

    println!("\nThese photos are found by now.");
    println!();
    println!("Federated by \t | Count");
    println!("------------ \t | -----");
    photos_by_federation.iter().for_each(|(k, v)| {
        println!("{k:?} \t | {}", v);
    });
    println!("---------");
    println!("Total: {} photos.", photos_by_federation.values().sum::<usize>());
    println!();
    println!("These photos will be migrated. Do you want to continue?");
    println!("Note that the file will be re-read, so if the file content is changed, you should re-run the utility.");
    println!("You can always redo this migration as long as the JSON files are intact.");
    println!();
    indoc::printdoc! {"
    
    2. Create Sqlite file
    ---------------------

    Iris will writee to the Sqlite database. Note that the database file itself should be already created,
    but the content is blank and this step will write to it.
    Do you want to continue?: [yN] "};
    std::io::stdout().flush()?;
    if !confirm_from_user()? {
        println!("Aborted.");
        return Ok(());
    }

    println!("Executing migration - this will take time!");
    println!("");
    println!("");
    let mut registry = ctx.registry.write().await;
    let mut migrated = 0usize;
    let mut skipped = 0usize;

    for (i, photo) in json_registry.dangerously_read_photo()?.enumerate() {
        let result = registry.insert_photo_reference(photo.clone()).await;

        match result {
            Ok(Some(())) => {
                println!("\x1b[A[{}/{}] Migrated {} ({:?})", i + 1, total_count, photo.id(), photo.origin.federator());
                migrated += 1;
            },
            Ok(None) => {
                println!("\x1b[A[{}/{}] Skipped  {} ({:?})", i + 1, total_count, photo.id(), photo.origin.federator());
                skipped += 1;
            },
            Err(err) => {
                println!("\x1b[A[{}/{}]  FAILED  {} ({:?})", i + 1, total_count, photo.id(), photo.origin.federator());
                println!("{}", err);
                println!();
            }
        }

        std::io::stdout().flush()?;
    }

    indoc::printdoc! {"
    Okay! The photo migration was successful.

        Migrated: {migrated}
        Skipped : {skipped} (the photo was already registered)
    "};

    Ok(())
}

fn confirm_from_user() -> anyhow::Result<bool> {
    let mut string = String::new();
    std::io::stdin()
        .read_line(&mut string)
        .context("Cannot read from stdin")?;

    Ok(string.to_lowercase().starts_with("y"))
}



