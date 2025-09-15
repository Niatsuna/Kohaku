use std::{error::Error, sync::Arc};

use tokio::sync::{Mutex, OnceCell};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::utils::scheduler::tasks::{Runnable, Task};

static SCHEDULER: OnceCell<Arc<Scheduler>> = OnceCell::const_new();
pub struct Scheduler {
    scheduler: Arc<Mutex<JobScheduler>>,
}

impl Scheduler {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            scheduler: Arc::new(Mutex::new(JobScheduler::new().await?)),
        })
    }

    /// Schedule a given task for the scheduler
    pub async fn add_task<T>(&self, task: T) -> Result<(), Box<dyn Error>>
    where
        T: Runnable + std::ops::Deref<Target = Task> + 'static + Send + Sync,
    {
        let task = Arc::new(task);
        let job = Job::new_async(&task.cron, {
            let task = Arc::clone(&task);
            move |uuid, scheduler| {
                let task = Arc::clone(&task);
                Box::pin(async move {
                    // Run task
                    task.run().await;

                    // Remove task if it should only run once
                    if task.run_once {
                        scheduler.remove(&uuid).await.unwrap();
                    }
                })
            }
        })?;

        let scheduler = self.scheduler.lock().await;
        scheduler.add(job).await?;
        Ok(())
    }

    /// Start scheduler
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let scheduler = self.scheduler.lock().await;
        scheduler.start().await?;
        Ok(())
    }
}

pub async fn init_scheduler() -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = Arc::new(Scheduler::new().await?);
    SCHEDULER
        .set(scheduler)
        .map_err(|_| "Scheduler already initilized")?;
    Ok(())
}

pub async fn get_scheduler() -> Arc<Scheduler> {
    SCHEDULER
        .get()
        .expect("Scheduler not initilized - call init_scheduler first")
        .clone()
}
