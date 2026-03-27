use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};

use crate::{Context, processor::{JobApplication, runner::run_image_processing}};

#[derive(Default)]
pub struct ProcessorQueue {
    jobs: Mutex<VecDeque<JobApplication>>,
    closed: Notify,
    notify: Notify,
}

impl ProcessorQueue {
    pub fn add_job(&self, job: JobApplication) {
        self.jobs.lock().unwrap().push_back(job);
        self.notify.notify_one();
    }

    pub fn pop_job(&self) -> Option<JobApplication> {
        let mut jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());

        jobs.pop_front()
    }
}

impl Drop for ProcessorQueue {
    fn drop(&mut self) {
        self.closed.notify_one();
    }
}

pub struct ProcessorRunner {
    ctx: Arc<Context>,
    runner_semaphore: Arc<Semaphore>,
}

impl ProcessorRunner {
    pub fn from_context(context: Arc<Context>) -> ProcessorRunner {
        ProcessorRunner {
            ctx: context,
            runner_semaphore: Arc::new(Semaphore::new(4)),
        }
    }

    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                _ = self.ctx.processor.queue.closed.notified() => { break Ok(()) },
                _ = self.ctx.processor.queue.notify.notified() => {
                    self.clone().drain_queue().await;
                }
            }
        }
    }

    async fn drain_queue(self: Arc<Self>) {
        while let Some(job) = self.ctx.processor.queue.pop_job() {
            let permit = Arc::clone(&self.runner_semaphore).acquire_owned().await.unwrap();
            let ctx = self.ctx.clone();

            tokio::spawn(async move {
                let _permit: OwnedSemaphorePermit = permit;
                match job {
                    JobApplication::Ping(_) => {},
                    JobApplication::ImageProcess(job) => {
                        if let Err(e) = run_image_processing(ctx, job).await {
                            tracing::error!("Image processing failed: {e}");
                        }
                    }
                }
            });
        }
    }
}
