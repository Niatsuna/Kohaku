# Scheduler
Kohaku features a built-in scheduler using the crate [`tokio_cron_scheduler`](https://github.com/mvniekerk/tokio-cron-scheduler).
This services allows for automatic processes to spawn at given times and follows a cron-like schedule.

It's available throughout the backend by calling `get_scheduler()` and follows a singleton pattern.

## Cron
[`tokio_cron_scheduler`](https://github.com/mvniekerk/tokio-cron-scheduler) uses [`croner-rust`](https://github.com/Hexagon/croner-rust) as its cron parser which has the following pattern and available properties
```
sec min hour day of month month day of week
*   *   *    *            *     *
```

- `*` indicates `ANY`, meaning for example that `0 0 12 * * *` will execute every day exactly one time at `12:00:00` (12 pm)
- comma-separated values are a list of values, meaning for example that `0 0 12,18 * * *` will execute every day exactly two times, once at `12:00:00` (12 pm) and a second time at `18:00:00` (6 pm)
- slash-separated values are steps, meaning for example that `*/10 * * * * *` executes every ten seconds
- `L` indicating the last day of the month
- `#` indicates the Nth occurence of the weekday
- `W` closest weekday
- default pattern is a logical-OR, meaning `0 0 0 1 * MON` would execute every monday and every first day of any month at `00:00:00` (12 am). To have a logical-AND a `+` can extend the functionalities, meaning `0 0 0 1 * +MON` would only execute if the first of any month is also a monday.


| Field        | Required | Allowed values  | Allowed special characters | Remarks                                                                                                         |
| ------------ | -------- | --------------- | -------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Seconds      | Optional | 0-59            | * , - /                    |                                                                                                                 |
| Minutes      | Yes      | 0-59            | * , - /                    |                                                                                                                 |
| Hours        | Yes      | 0-23            | * , - /                    |                                                                                                                 |
| Day of Month | Yes      | 1-31            | * , - / ? L W              |                                                                                                                 |
| Month        | Yes      | 1-12 or JAN-DEC | * , - /                    |                                                                                                                 |
| Day of Week  | Yes      | 0-7 or SUN-MON  | * , - / ? # L +            | 0 to 6 are Sunday to Saturday<br>7 is Sunday, the same as 0<br># is used to specify nth occurrence of a weekday |

For more information: [`croner-rust`](https://github.com/Hexagon/croner-rust)

_(Source: [`tokio_cron_scheduler`](https://github.com/mvniekerk/tokio-cron-scheduler) & [`croner-rust`](https://github.com/Hexagon/croner-rust))_

## Tasks
[`Tasks`](../server/src/utils/scheduler/tasks.rs) are build via a [`TaskBuilder`](../server/src/utils/scheduler/tasks.rs#L53), allowing for easy construction.

| Field | Required | Description | Method |
| ----- | -------- | ----------- | ------ |
| `name` | Yes | Logging purposes | --
| `cron` | Yes | Schedule in which the task should be executed | [`schedule(cron : &str)`]()
| `handler` | Yes | Actual function to be executed | [`handler(f : TaskFn)`]()
| `remaining_runs` | No | How often the task should be executed, if not set default is infinite (indicated by `-1`) | [`run_once()`]() or [`run_times(times: i32)`]()

The construction can be finished via `.build()` at the end and if successful, will return the resulting [`Task`]() struct.
If the construction fails, a [`KohakuError::TaskBuilderError`]() will be returned.

The task can then be scheduled via `scheduler.add_task(task).await`.

### Examples
```rust
// Task will execute once
let task = Task::builder("test")
    .schedule("* * * * * *")
    .run_once()
    .handler(|| async { Ok(())})
    .build()?;
scheduler.add_task(task).await;

// Task will execute every 10 seconds for 5 times
let task = Task::builder("test")
  .schedule("*/10 * * * * *")
  .run_times(5)
  .handler(|| async { Ok(())})
  .build()?;
scheduler.add_task(task).await;

// Task will execute every day at 12:00:00 (12 pm) infinitely
let task = Task::builder("test")
  .schedule("0 0 12 * * *")
  .handler(|| async { Ok(())})
  .build()?;
scheduler.add_task(task).await;
```
