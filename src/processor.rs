mod runner;
mod protocol;
mod image;
pub mod config;
pub mod queue;

use std::sync::Arc;

pub use protocol::*;

use crate::{Context, ingest::model::Identifier, processor::{config::ImageProcessConfig, queue::{ProcessorQueue, ProcessorRunner}}};

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

pub async fn run(ctx: Arc<Context>) -> anyhow::Result<()> {
    Arc::new(ProcessorRunner::from_context(ctx))
        .start()
        .await
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

