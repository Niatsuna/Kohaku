use std::{error::Error, sync::Arc};

use tokio::sync::{Mutex, OnceCell};
use tokio_cron_scheduler::{job::job_data::Uuid, Job, JobScheduler, JobSchedulerError};

pub mod tasks;
use crate::utils::{
    error::KohakuError,
    scheduler::tasks::{Runnable, Task},
};

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
    pub async fn add_task<T>(&self, task: T) -> Result<Uuid, KohakuError>
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
        })
        .map_err(KohakuError::SchedulerError)?;

        let scheduler = self.scheduler.lock().await;
        let uuid = scheduler
            .add(job)
            .await
            .map_err(KohakuError::SchedulerError)?;
        Ok(uuid.into())
    }

    /// Start scheduler
    pub async fn start(&self) -> Result<(), KohakuError> {
        let scheduler = self.scheduler.lock().await;
        scheduler
            .start()
            .await
            .map_err(KohakuError::SchedulerError)?;
        Ok(())
    }
}

pub async fn init_scheduler() -> Result<(), KohakuError> {
    let scheduler = Arc::new(
        Scheduler::new()
            .await
            .map_err(|_| KohakuError::SchedulerError(JobSchedulerError::CantInit))?,
    );
    SCHEDULER
        .set(scheduler)
        .map_err(|_| KohakuError::SchedulerError(JobSchedulerError::CantInit))?;
    Ok(())
}

pub async fn get_scheduler() -> Arc<Scheduler> {
    SCHEDULER
        .get()
        .expect("Scheduler not initialized - call init_scheduler first")
        .clone()
}
