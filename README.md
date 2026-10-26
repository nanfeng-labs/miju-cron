# miju-cron

Schedule recurring tasks for miju Workers using cron expressions, English routines, a fluent builder, or a custom `Schedule<Tz>` implementation.

```rust,no_run
# #[cfg(feature = "cron")]
# mod example {
use cron::Schedule;
use miju::{layers::retry::RetryPolicy, prelude::*};
use miju_cron::{CronScheduler, Tick};
use std::str::FromStr;

async fn handle_tick(tick: Tick, data: Data<usize>) {
    assert!(*data == 42usize);
    println!("Tick received at {}", tick.get_timestamp());
}

#[tokio::main]
async fn main() {
    let schedule = Schedule::from_str("@daily").unwrap();

    let worker = WorkerBuilder::new("morning-cereal")
        .backend(CronScheduler::new(schedule))
        .retry(RetryPolicy::retries(5))
        .data(42usize)
        .build(handle_tick);

    worker.run().await.unwrap();
}
# }
```

## Schedules and features

The default features are `cron` and `serde`. `chrono` exposes Chrono timezones, `builder` enables the fluent schedule builder, `english` enables `EnglishRoutine`, and `jiff` enables `JiffSchedule`. `docsrs` generates the feature reference. The current dependency set requires Rust 1.89.

`Tick<Tz>` stores a UTC UNIX timestamp in seconds. The timezone type describes how the schedule calculates that timestamp. `Config` serializes `schedule` and `timezone` as strings and parses both on deserialization. Tick serialization includes `timestamp` and `_marker`.

`CronScheduler` is an ephemeral Backend. It caches the next tick, waits with a runtime-independent timer, emits a task with a time-derived ULID, and prefetches the next tick. `into_stream()` yields ticks without a Worker. Cron and English schedules calculate their next occurrence from the current clock; Builder retains its previous tick. Tokio's paused clock does not advance the scheduler timer.

## Examples

- `basic`: cron expressions, retries, and shared data.
- `builder`: fluent schedule construction and Worker execution.
- `timezone`: timezone-typed ticks and the builder (see its limitations below).
- `english`: English parsing with a local timezone and retries.
- `config`: serialized Worker/backend/middleware configuration.
- `custom`: implementing a local-time daily routine with `Schedule`. This example retains a missing-data limitation: its handler requests `Data<usize>`, but the Worker does not inject that value, so the handler fails extraction when a tick arrives.

Run `cargo run --example basic`, or select the corresponding feature, for example `cargo run --example builder --features builder`. These examples run recurring Workers and normally continue until interrupted.

A scheduler can be composed with another backend using `pipe_to`. Only tasks already emitted into that backend can be stored; the scheduler itself does not persist rules, replay downtime, elect a leader, or promise exactly-once execution. SQLite persistence and workflow examples are pending availability of the matching storage dependency.

## Current behavioral limitations

- An expired tick encountered before timer creation returns `OutOfRange` and remains cached. Polling again returns the same error. A timer already created can yield its task after the planned time.
- `poll_close` acknowledges immediately and does not reset the scheduler. Drop the scheduler to cancel its timer. Clone and timezone replacement discard the cached tick and timer while retaining the current schedule state.
- Builder calculates in UTC even when another timezone is supplied. `every(n).minutes()` without an `at` time advances by `n * n` minutes; with `at`, it advances to the next matching minute. A weekday without `.week()` has no unit and returns no tick. Zero intervals do not advance; sufficiently large minute intervals overflow the intermediate `u32` multiplication (panic in debug builds).
- With miju-core 0.1.0, a finite scheduler stream ends, but a Worker reaching that end does not automatically complete shutdown. Explicit Worker stop works. The regression test for automatic completion remains failing until the dependency is fixed.

## License

MIT; see `LICENSE`.
