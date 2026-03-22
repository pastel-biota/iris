use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tokio::sync::{Notify, Semaphore};

use crate::{Context, processor::{JobApplication, ProcessorContext, runner::run_image_processing}};

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

    fn notify_job(&self) {
        self.notify.notify_one();
    }

    fn notify_close(&self) {
        self.closed.notify_one();
    }
}

impl Drop for ProcessorQueue {
    fn drop(&mut self) {
        self.closed.notify_one();
    }
}

pub struct ProcessorRunner {
    ctx: Arc<Context>,
    runner_semaphore: Semaphore,
}

impl ProcessorRunner {
    pub fn from_context(context: Arc<Context>) -> ProcessorRunner {
        ProcessorRunner {
            ctx: context,
            runner_semaphore: Semaphore::new(4),
        }
    }

    pub async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        loop {
            println!("Polling");
            tokio::select! {
                _ = self.ctx.processor.queue.closed.notified() => { break Ok(()) },
                _ = self.ctx.processor.queue.notify.notified() => {
                    let cloned_self = self.clone();
                    tokio::spawn(async move { cloned_self.handle_notify().await; });
                }
            }
        }
    }

    async fn handle_notify(&self) {
        let _permit = self.runner_semaphore.acquire().await.unwrap();

        while let Some(job) = {
            self.ctx.processor.queue
                .jobs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pop_front()
        } {
            match job {
                JobApplication::Ping(_) => {},
                JobApplication::ImageProcess(job) => {
                    run_image_processing(self.ctx.clone(), job).await.unwrap();
                }
            }
        }
    }

    async fn run(&self, job: JobApplication) {
        println!("Working on: {job:?} ...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        println!("  ... Done: {job:?}");
    }
}
