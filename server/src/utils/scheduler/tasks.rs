use std::{future::Future, pin::Pin, sync::{Arc, atomic::{AtomicI32, Ordering}}};

use tracing::{error, info};

use crate::utils::error::KohakuError;

pub type TaskFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), KohakuError>> + Send>> + Send + Sync>;

/// Scheduled Task that executes the given function on the given schedule.
pub struct Task {
    /// Name for logging purposes
    pub name: String,
    /// Cron-like schedule (see [`tokio_cron_scheduler`])
    pub cron: String,
    /// Counter how often this task should be executed until removal (-1 = infinite)
    pub remaining_runs: Arc<AtomicI32>,
    /// Actual function to execute on given schedule
    pub handler: TaskFn
}

impl Task {
    pub fn builder(name: impl Into<String>) -> TaskBuilder {
        TaskBuilder::new(name)
    }

    /// Decrements and checks the current lifespan of the given task.
    /// 
    /// # Returns:
    /// - [`bool`] indicating if the Task should be removed or not. Please note that a set lifespan of `-1` will be counted as `infinite` and will result in no removal.
    pub fn check_lifespan(&self) -> bool {
        let current = self.remaining_runs.load(Ordering::SeqCst);

        if current == -1 {
            return false;
        }

        let new_value = self.remaining_runs.fetch_sub(1, Ordering::SeqCst) - 1;
        new_value <= 0
    }
}

/// Builder for [`Task`].
pub struct TaskBuilder {
    name: String,
    cron: Option<String>,
    runs: i32,
    handler: Option<TaskFn>
}

impl TaskBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cron: None,
            runs: -1,
            handler: None
        }
    }

    /// Sets the schedule using a `cron` structure (see [`tokio_cron_scheduler`])
    pub fn schedule(mut self, cron: impl Into<String>) -> Self {
        self.cron = Some(cron.into());
        self
    }

    /// Sets the counter to 1 (Default: Infinite)
    pub fn run_once(mut self) -> Self {
        self.runs = 1;
        self
    }

    /// Sets the counter to `times` (Default: Infinite)
    pub fn run_times(mut self, times: i32) -> Self {
        self.runs = times;
        self
    }

    /// Setting the function that should be exectued on schedule
    pub fn handler<F, Fut>(mut self, f: F) -> Self 
    where 
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), KohakuError>> + Send + 'static,
    {
        self.handler = Some(Arc::new(move || Box::pin(f())));
        self
    }

    /// Builds the given task.
    /// 
    /// Requirement are `cron` and `handler`.
    /// 
    /// # Returns
    /// A [`Result`] which is either:
    /// - [`Ok`] : Constructed [`Task`]
    /// - [`Err`] : A [`KohakuError::TaskBuilderError`] if either `cron` or `handler` were not set
    pub fn build(self) -> Result<Task, KohakuError> {
        let cron = self.cron.ok_or(KohakuError::TaskBuilderError("Schedule (cron) is required".to_string()))?;
        let handler = self.handler.ok_or(KohakuError::TaskBuilderError("Handler function is required".to_string()))?;

        Ok(Task {
            name: self.name,
            cron,
            remaining_runs: Arc::new(AtomicI32::new(self.runs)),
            handler
        })
    }
}

pub trait Runnable: Send + Sync {
    fn run(&self) -> impl Future<Output = Result<(), KohakuError>> + Send;
}

impl Runnable for Task {
    /// Runs the scheduled tasks function.
    /// Result will be logged using [`tracing`] on the server side.
    /// 
    /// # Returns
    /// A [`Result`] which is either:
    /// - [`Ok`] : Task executed without any errors
    /// - [`Err`] : A [`KohakuError::TaskExecutionError`] holding the error type which lead to a failure
    async fn run(&self) -> Result<(), KohakuError> {
        match (self.handler)().await {
            Ok(_) => {
                info!("[ Task - {} ] Execution finished!", self.name);
                Ok(())
            },
            Err(e) => {
                error!("[ Task - {} ] Failure detected: {}", self.name, e);
                Err(KohakuError::TaskExecutionError(Box::new(e)))
            }
        }
    }
}