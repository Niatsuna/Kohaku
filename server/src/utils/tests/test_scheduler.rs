use std::{
    sync::{
        atomic::{AtomicI32, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use rstest::rstest;

use crate::utils::{
    error::KohakuError,
    scheduler::{
        get_scheduler, init_scheduler,
        tasks::{Runnable, Task},
    },
};

#[test]
fn test_task_builder_basic() {
    let name = "test";
    let cron = "* * * * * *";

    let task = Task::builder(name)
        .schedule(cron)
        .handler(|| async { Ok(()) })
        .build();

    assert!(task.is_ok());
    let task = task.unwrap();
    assert_eq!(task.name, name);
    assert_eq!(task.cron, cron);
    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), -1);
}

#[rstest]
#[case(1)]
#[case(10)]
fn test_task_builder_diff_runs(#[case] amount: i32) {
    let tb = Task::builder("test")
        .schedule("*/1 * * * * *")
        .handler(|| async { Ok(()) });

    let task: Result<Task, KohakuError>;
    if amount == 1 {
        task = tb.run_once().build();
    } else {
        task = tb.run_times(amount).build();
    }

    assert!(task.is_ok());
    let task = task.unwrap();
    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), amount);
}

#[test]
fn test_task_builder_missing_schedule() {
    let task = Task::builder("test").handler(|| async { Ok(()) }).build();
    assert!(task.is_err());
    assert!(
        matches!(task, Err(KohakuError::TaskBuilderError(t)) if t == "Schedule (cron) is required".to_string())
    );
}

#[test]
fn test_task_builder_missing_handler() {
    let task = Task::builder("test").schedule("* * * * * *").build();
    assert!(task.is_err());
    assert!(
        matches!(task, Err(KohakuError::TaskBuilderError(t)) if t == "Handler function is required".to_string())
    );
}

#[tokio::test]
async fn test_task_decrement_count_infinite() {
    let task = Task::builder("test")
        .schedule("*/1 * * * * *")
        .handler(|| async { Ok(()) })
        .build();
    assert!(task.is_ok());
    let task = task.unwrap();

    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), -1);
    assert!(!task.check_lifespan());
    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), -1);
}

#[tokio::test]
async fn test_task_decrement_count_once() {
    let task = Task::builder("test")
        .schedule("*/1 * * * * *")
        .handler(|| async { Ok(()) })
        .run_once()
        .build();
    assert!(task.is_ok());
    let task = task.unwrap();

    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), 1);
    assert!(task.check_lifespan());
    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), 0);
    assert!(task.check_lifespan());
    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[rstest]
#[case(2)]
#[case(5)]
#[case(10)]
async fn test_task_decrement_count_finite_times(#[case] amount: i32) {
    let task = Task::builder("test")
        .schedule("*/1 * * * * *")
        .handler(|| async { Ok(()) })
        .run_times(amount)
        .build();
    assert!(task.is_ok());
    let task = task.unwrap();

    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), amount);
    assert!(!task.check_lifespan());
    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), amount - 1);

    if amount == 2 {
        assert!(task.check_lifespan());
    } else {
        assert!(!task.check_lifespan());
    }
    assert_eq!(task.remaining_runs.load(Ordering::SeqCst), amount - 2);
}

#[tokio::test]
async fn test_task_execution() {
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    let task = Task::builder("test")
        .schedule("*/1 * * * * *")
        .run_once()
        .handler(move || {
            let count = counter_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .build();

    assert!(task.is_ok());
    let task = task.unwrap();

    let res = task.run().await;
    assert!(res.is_ok());

    let c = counter.load(Ordering::SeqCst);
    assert_eq!(c, 1);
}

#[tokio::test]
async fn test_scheduler_task_run_once() {
    let _ = init_scheduler().await;
    let scheduler = get_scheduler().await;
    let _ = scheduler.start().await;

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    let task = Task::builder("test")
        .schedule("*/1 * * * * *")
        .run_once()
        .handler(move || {
            let count = counter_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .build();

    assert!(task.is_ok());
    let task = task.unwrap();
    let res = scheduler.add_task(task).await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_secs(3)).await;

    let c = counter.load(Ordering::SeqCst);
    assert_eq!(c, 1);
}

#[tokio::test]
#[rstest]
#[case(2)]
#[case(3)]
async fn test_scheduler_task_run_times(#[case] amount: i32) {
    let _ = init_scheduler().await;
    let scheduler = get_scheduler().await;
    let _ = scheduler.start().await;

    let counter = Arc::new(AtomicI32::new(0));
    let counter_clone = counter.clone();
    let task = Task::builder("test")
        .schedule("* * * * * *")
        .run_times(amount)
        .handler(move || {
            let count = counter_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .build();

    assert!(task.is_ok());
    let task = task.unwrap();
    let rem_runs = task.remaining_runs.clone();
    let res = scheduler.add_task(task).await;
    assert!(res.is_ok());

    let waittime = (amount as u64) + 3;
    tokio::time::sleep(Duration::from_secs(waittime)).await;

    let c = counter.load(Ordering::SeqCst);
    let diff = amount - c;

    // Allow for an error range of 1 count based on scheduler startup and schedule time based on system
    assert!(
        diff <= 1,
        "Expected {} executions, got {} (Diff Threshold : 1)",
        amount,
        c
    );
    assert_eq!(rem_runs.load(Ordering::SeqCst), diff);
}
