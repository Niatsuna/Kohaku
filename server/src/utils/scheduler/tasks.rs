use std::future::Future;

pub struct Task {
    // Name of task for logging purposes
    pub name: String,
    // Schedule (see tokio_cron_scheduler)
    pub cron: String,
    // How often the task should be repeated. (-1 = Infinite)
    pub run_once: bool,
}

impl Task {
    pub fn new(name: &str, cron: &str, run_once: bool) -> Self {
        Self {
            name: name.to_string(),
            cron: cron.to_string(),
            run_once,
        }
    }
}

pub trait Runnable: Send + Sync {
    fn run(&self) -> impl Future<Output = ()> + Send;
}
/// Use this macro to quickly implement the foundation of your task!
///
/// Example:
/// ```
///   pub struct MyTask(Task);
///
///   impl MyTask {
///     pub fn new() -> Self {
///        Self(Task::new("Example", "0,30 * * * * *", false))
///     }
///     async fn execute(&self) -> Result<(), String> {
///         info!("Example-Task-Execution");
///         Ok(())
///     }
///   }
///   impl_task_wrapper!(MyTask);
/// ```
///
#[macro_export]
macro_rules! impl_task_wrapper {
    ($($t:ty),*) => {
        $(
            impl std::ops::Deref for $t {
                type Target = Task;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl $crate::utils::scheduler::tasks::Runnable for $t {
              async fn run(&self) -> () {
                if let Err(e) = self.execute().await {
                  error!("[ Task - {} ] - Failure detected: {e}", self.0.name);
                  return;
                }
                info!("[ Task - {} ] - Done!", self.0.name);
              }
            }
        )*
    }
}
