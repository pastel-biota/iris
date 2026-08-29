use std::{sync::Arc, time::Duration};

use anyhow::Context as _;
use notify::{
    EventKind,
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
};
use notify_debouncer_full::{new_debouncer, notify::RecursiveMode};

use crate::{Context, repository::io::ScopedPath};

pub async fn reindex_everything(ctx: Arc<Context>) {
    ctx.indexer.driver.fully_reindex().await;
}

pub async fn watch_file(ctx: Arc<Context>) -> anyhow::Result<()> {
    let scope_path = &ctx.scope;

    tracing::info!(
        "Wathcing the directory content change at: {}",
        scope_path.display()
    );

    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = new_debouncer(Duration::from_secs(3), None, tx)
        .context("while creating the debounced file watcher")?;

    watcher
        .watch(scope_path.use_path(), RecursiveMode::Recursive)
        .context("while starting to watch the directory")?;

    while let Ok(events) = rx.recv() {
        let events = match events {
            Ok(events) => events,
            Err(errs) => {
                tracing::warn!(
                    "One or more errors were reported while watching. Continuing to watch anyway"
                );
                errs.iter().for_each(|err| tracing::warn!("{err}"));
                continue;
            }
        };

        for event in events {
            let paths = event.paths
                .iter()
                .flat_map(|path| {
                    let scope_path = ScopedPath::from_within(&scope_path, path);

                    if let Err(err) = &scope_path {
                        tracing::warn!("The change was detected to this path but this file will be ignored\n  path: {}\n  reason: {}", path.display(), err);
                    }

                    scope_path.ok()
                })
                .filter(|path| path.use_path().extension().is_none_or(|ext| ext == "json"))
                .collect::<Vec<_>>();

            if paths.is_empty() {
                continue;
            }

            let path_display = paths
                .iter()
                .map(|path| format!("{}", path.use_path().display()))
                .collect::<Vec<_>>();

            match event.kind {
                EventKind::Create(CreateKind::File)
                | EventKind::Create(CreateKind::Folder)
                | EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Metadata(_))
                | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    tracing::debug!("The file/folder is touched: {:?}", path_display);

                    let mut files = vec![];
                    for path in paths {
                        files.extend(resolve_changed_files(path).unwrap());
                    }

                    tracing::debug!("These files will be processed: {:#?}", files);

                    process_modified_file(files, ctx.clone()).await;
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    tracing::warn!(
                        "{}",
                        indoc::formatdoc! {"
                            The file/folder has been moved out of the scope directory:
                            - {}

                            Iris currently does not support live update for removal of the file/folder.
                            It is highly recommended to remove the Sqlite file and rebuild the index.
                        ", path_display.join("\n- ") }
                    );
                    continue;
                }
                EventKind::Remove(RemoveKind::File | RemoveKind::Folder) => {
                    tracing::warn!(
                        "{}",
                        indoc::formatdoc! {"
                            The file/folder has been removed:
                            - {}

                            Iris currently does not support live update for removal of the file/folder.
                            It is highly recommended to remove the Sqlite file and rebuild the index.
                        ", path_display.join("\n- ") }
                    );
                    continue;
                }
                EventKind::Create(CreateKind::Any | CreateKind::Other)
                | EventKind::Remove(RemoveKind::Any | RemoveKind::Other)
                | EventKind::Modify(ModifyKind::Name(
                    RenameMode::Any | RenameMode::Both | RenameMode::Other,
                ))
                | EventKind::Modify(ModifyKind::Any | ModifyKind::Other) => {
                    tracing::warn!(
                        "Unknown event for these path:\n- {}\n\nThis change was not being processed. Sqlite index might be stale.",
                        path_display.join("\n- "),
                    );
                }
                EventKind::Any | EventKind::Access(_) | EventKind::Other => {}
            }
        }
    }

    todo!();
}

fn resolve_changed_files(path: ScopedPath) -> anyhow::Result<Vec<ScopedPath>> {
    if !path.exists() {
        anyhow::bail!(
            "non-existent path's processing is currently not implemented, please rebuild the Sqlite index"
        );
    }

    let metadata = path
        .use_path()
        .metadata()
        .context("while retrieving the changed path's metadata")?;

    if metadata.is_file() {
        return Ok(
            if path.use_path().extension().is_some_and(|ext| ext == "json") {
                vec![path]
            } else {
                vec![]
            },
        );
    }

    if !metadata.is_dir() {
        anyhow::bail!(
            "Expected to be the directory given that this is not the file, but it is not a file nor a directory??"
        );
    }

    let mut files = Vec::new();

    for walk in path.walk_files() {
        let walk = walk.context("while scanning the changed directory")?;

        if walk
            .path
            .use_path()
            .extension()
            .is_some_and(|ext| ext == "json")
        {
            files.push(walk.path);
        }
    }

    Ok(files)
}

async fn process_modified_file(path: Vec<ScopedPath>, ctx: Arc<Context>) {
    for path in path {
        ctx.indexer.driver.index_file(&path).await.ok();
    }
}
