use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use serial_test::serial;

use crate::{
    impl_task_wrapper,
    utils::scheduler::{
        scheduler::{get_scheduler, init_scheduler, Scheduler},
        tasks::Task,
    },
};

#[tokio::test]
#[should_panic]
async fn test_scheduler_not_initilized_before_get() {
    let _ = get_scheduler().await;
}

#[tokio::test]
async fn test_scheduler_singleton() {
    // #1 Test that first initialization succeeds and second fails
    assert!(init_scheduler().await.is_ok());
    assert!(init_scheduler().await.is_err());

    // #2 Test if getter returns same instance
    let s1 = get_scheduler().await;
    let s2 = get_scheduler().await;
    assert!(Arc::ptr_eq(&s1, &s2))
}

// ------------------------------------------------------------------------

static COUNTER: Mutex<Option<Arc<AtomicUsize>>> = Mutex::new(None);

struct TestTask(Task);

impl TestTask {
    pub fn new(run_once: bool) -> Self {
        Self(Task::new("TestTask", "*/1 * * * * *", run_once))
    }

    async fn execute(&self) -> Result<(), String> {
        let counter = COUNTER.lock().unwrap();
        let counter = counter.as_ref().expect("Counter not initialized");
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl_task_wrapper!(TestTask);

#[tokio::test]
async fn test_add_task() {
    let task1 = TestTask::new(true);
    let task2 = TestTask::new(true);

    let scheduler = Scheduler::new().await.unwrap();
    // #1 Test if task is added if scheduler has not started yet
    assert!(scheduler.add_task(task1).await.is_ok());
    let _ = scheduler.start().await;

    // #2 Test if task is added if scheduler has already started
    assert!(scheduler.add_task(task2).await.is_ok());
}

#[tokio::test]
#[serial]
async fn test_execute_task_once() {
    let counter = Arc::new(AtomicUsize::new(0));
    *COUNTER.lock().unwrap() = Some(counter.clone());

    let task = TestTask::new(true);

    let scheduler = Scheduler::new().await.unwrap();
    let _ = scheduler.add_task(task).await;
    let _ = scheduler.start().await;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let count = counter.load(Ordering::SeqCst);
    assert_eq!(
        count, 1,
        "Task should run exactly once, but ran {} times",
        count
    );
}

#[tokio::test]
#[serial]
async fn test_execute_task_multiple_times() {
    let counter = Arc::new(AtomicUsize::new(0));
    *COUNTER.lock().unwrap() = Some(counter.clone());

    let task = TestTask::new(false);

    let scheduler = Scheduler::new().await.unwrap();
    let _ = scheduler.add_task(task).await;
    let _ = scheduler.start().await;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let count = counter.load(Ordering::SeqCst);
    assert!(
        count > 1,
        "Task should run multiple times, but ran {} time(s)",
        count
    );
}
