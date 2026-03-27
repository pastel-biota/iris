mod runner;
mod protocol;
pub mod config;
pub mod queue;

use std::sync::Arc;

pub use protocol::*;

use crate::{
    Context,
    event::{Event, EventReceiver},
    model::Identifier,
    processor::{config::ImageProcessConfig, queue::{ProcessorQueue, ProcessorRunner}},
};

pub struct ProcessorContext {
    pub config: ImageProcessConfig,
    queue: Arc<ProcessorQueue>,
}

impl ProcessorContext {
    pub fn new(config: ImageProcessConfig) -> Self {
        Self {
            config,
            queue: Arc::new(ProcessorQueue::default()),
        }
    }
}

pub async fn run(ctx: Arc<Context>, event_rx: EventReceiver) -> anyhow::Result<()> {
    let runner = Arc::new(ProcessorRunner::from_context(ctx.clone()));

    tokio::try_join!(
        async { runner.start().await },
        async { listen_events(event_rx, &ctx).await },
    )?;

    Ok(())
}

async fn listen_events(
    mut rx: EventReceiver,
    ctx: &Arc<Context>,
) -> anyhow::Result<()> {
    while let Some(event) = rx.recv().await {
        match event {
            Event::PhotoRegistered { photo_id } => {
                tracing::info!("Received PhotoRegistered event for {photo_id}, enqueuing resize jobs");
                for image_id in ctx.processor.config.sizes.keys() {
                    register_resize(&ctx.processor, photo_id.clone(), image_id);
                }
            }
        }
    }

    Ok(())
}

pub fn register_resize(ctx: &ProcessorContext, photo_id: Identifier, image_id: &str) {
    let target = ctx.config.sizes.get(image_id).unwrap();

    ctx.queue.add_job(JobApplication::ImageProcess(
            ImageProcessJob {
                photo_id,
                image_id: image_id.to_string(),
                target: target.clone()
            }
    ));
}
