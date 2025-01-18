use std::error::Error;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Scheduled function for a given time.
///
/// ## Fields
/// - `name` - Name for logging purposes. (Development)
/// - `cron` - Cron-like string for scheduling the task. (Tokio-cron-scheduler)
/// - `func` - Async function that should be called.
/// - `once` - Flag if the task should return after the first trigger.
#[derive(Clone)]
pub struct Task<F>
where
    F: Future,
{
    pub name: String,
    pub cron: String,
    pub func: fn() -> F,
    pub once: bool,
}

impl<F> Task<F>
where
    F: Future,
{
  pub fn new(name : &str, cron : &str, func : fn() -> F, once: bool) -> Self {
    let name = String::from(name);
    let cron = String::from(cron);
    Task{name, cron, func, once}
  } 
}

/// Scheduler for Tasks.
///
/// ## Functions
/// - `schedule_task` - Schedules a task with its given parameters.
/// - `start` - Starts the scheduler.
pub struct Scheduler {
    scheduler: Arc<Mutex<JobScheduler>>,
    is_running: Arc<Mutex<bool>>,
}

impl Scheduler {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let scheduler = JobScheduler::new().await?;
        Ok(Self {
            scheduler: Arc::new(Mutex::new(scheduler)),
            is_running: Arc::new(Mutex::new(false)),
        })
    }

    pub async fn add_task<F, O>(&self, task: Task<F>)
    where
        F: Future<Output = O> + Send + 'static,
        O: Send + 'static,
    {
        let func = task.func.clone();
        let once = task.once.clone();
        let job = Job::new_async(&task.cron, move |uuid, scheduler| {
            Box::pin(async move {
                func().await;

                if once {
                    scheduler.remove(&uuid).await.unwrap();
                }
            })
        })
        .unwrap();

        let scheduler = self.scheduler.lock().await;
        scheduler.add(job).await.unwrap();
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let scheduler = self.scheduler.lock().await;
        let mut is_running = self.is_running.lock().await;

        if !*is_running {
            scheduler.start().await?;
            *is_running = true;
        }

        Ok(())
    }
}
