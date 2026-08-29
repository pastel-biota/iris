use std::sync::Arc;

use crate::{Context, event::EventReceiver};

pub struct RunServerResourcees {
    pub ctx: Arc<Context>,
    pub event_rx: EventReceiver,
}

pub async fn run_server(resources: RunServerResourcees) -> anyhow::Result<()> {
    crate::index::watch::reindex_everything(resources.ctx.clone()).await;

    tokio::try_join!(
        async {
            tokio::spawn(crate::infra::api::run(resources.ctx.clone()))
                .await
                .unwrap()
        },
        async {
            tokio::spawn(crate::index::watch::watch_file(resources.ctx.clone()))
                .await
                .unwrap()
        },
        async {
            tokio::spawn(crate::processor::run(
                resources.ctx.clone(),
                resources.event_rx,
            ))
            .await
            .unwrap()
        },
    )
    .unwrap();

    Ok(())
}
