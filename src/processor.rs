use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tokio::sync::{Notify, Semaphore};

mod runner;

#[derive(Default)]
pub struct ProcessorContext {
    jobs: Arc<Mutex<VecDeque<JobApplication>>>,
    closed: Arc<Notify>,
    notify: Arc<Notify>,
}

impl ProcessorContext {
    pub fn add_job(&self, job: JobApplication) {
        self.jobs.lock().unwrap().push_back(job);
        self.notify.notify_one();
    }

    pub fn notify_job(&self) {
        self.notify.notify_one();
    }

    pub fn notify_close(&self) {
        self.closed.notify_one();
    }
}

#[derive(Debug)]
pub struct JobApplication {
    pub id: usize,
}

pub struct ProcessorRunner {
    jobs: Arc<Mutex<VecDeque<JobApplication>>>,
    closed: Arc<Notify>,
    notify: Arc<Notify>,
    runner_semaphore: Semaphore,
}

impl ProcessorRunner {
    pub fn from_context(context: &ProcessorContext) -> ProcessorRunner {
        ProcessorRunner {
            jobs: context.jobs.clone(),
            closed: context.closed.clone(),
            notify: context.notify.clone(),
            runner_semaphore: Semaphore::new(4),
        }
    }

    pub async fn start(self: Arc<Self>) {
        loop {
            println!("Polling");
            tokio::select! {
                _ = self.closed.notified() => { break },
                _ = self.notify.notified() => {
                    let cloned_self = self.clone();
                    tokio::spawn(async move { cloned_self.handle_notify().await; });
                }
            }
        }
    }

    async fn handle_notify(&self) {
        let _permit = self.runner_semaphore.acquire().await.unwrap();

        while let Some(job) = {
            self.jobs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pop_front()
        } {
            println!("Working on: {job:?} ...");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            println!("  ... Done: {job:?}");
        }
    }
}
