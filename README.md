<p align="center">
  <img src="assets/miju-cron-header.svg" alt="miju-cron" width="480" />
</p>

<h1 align="center">miju-cron</h1>

<div align="center">
  <strong>Recurring task scheduling for Rust, built for miju Workers</strong>
</div>

<br />

<div align="center">
  <a href="https://crates.io/crates/miju-cron">
    <img src="https://img.shields.io/crates/v/miju-cron.svg" alt="Crates.io version" />
  </a>
  <a href="https://github.com/nanfeng-labs/miju-cron/actions/workflows/ci.yaml">
    <img src="https://github.com/nanfeng-labs/miju-cron/actions/workflows/ci.yaml/badge.svg?branch=main" alt="CI" />
  </a>
  <a href="Cargo.toml">
    <img src="https://img.shields.io/badge/Rust-1.89%2B-orange.svg" alt="Rust 1.89+" />
  </a>
  <a href="https://github.com/nanfeng-labs/miju">
    <img src="https://img.shields.io/badge/built%20for-miju-FF9A24.svg" alt="Built for miju" />
  </a>
</div>

<br />

<div align="center">
  <a href="README.zh-CN.md">简体中文</a>
</div>

## Features

- **Multiple schedule formats**: cron expressions, English routines, a fluent builder, and jiff schedules.
- **Typed ticks and timezones**: pass `Tick<Tz>` to handlers and use Chrono or jiff timezone support where the selected adapter supports it.
- **Custom schedules**: implement `Schedule<Tz>` to choose when the next task should run.
- **Worker integration**: combine scheduled tasks with miju's shared data, retries, and middleware.
- **Direct tick streams**: use `into_stream()` to consume scheduled ticks without a Worker.
- **Optional serialization**: encode schedule configuration and ticks with Serde.

## Installation

Use the current source from this repository with the published miju dependencies:

```toml
[dependencies]
miju-cron = { git = "https://github.com/nanfeng-labs/miju-cron.git", branch = "main" }
miju = "0.1.0"
cron = "0.17"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The quick start imports `cron::Schedule`, so `cron` is an explicit application dependency. Commit your application's `Cargo.lock` to retain the resolved Git revision.

Use **Rust 1.89 or later** for the current dependency set, including ULID task IDs. The package manifest's `rust-version` is still 1.85; CI checks the complete feature set with Rust 1.89.

## Quick start

Run a handler every ten seconds, using a six-field cron expression with seconds first:

```rust,no_run
# #[cfg(feature = "cron")]
# mod example {
use cron::Schedule;
use miju::prelude::*;
use miju_cron::{CronScheduler, Tick};
use std::str::FromStr;

async fn handle_tick(tick: Tick) {
    println!("Scheduled timestamp: {}", tick.get_timestamp());
}

#[tokio::main]
async fn main() -> Result<(), BoxDynError> {
    let schedule = Schedule::from_str("*/10 * * * * *")?;

    WorkerBuilder::new("recurring-task")
        .backend(CronScheduler::new(schedule))
        .build(handle_tick)
        .run()
        .await?;

    Ok(())
}
# }
```

This is a recurring Worker: `run()` waits until the Worker stops. Interrupt the process to end this minimal example; applications can request shutdown through `WorkerContext::stop()`.

For retries and shared handler data, see [`examples/basic.rs`](examples/basic.rs).

## Feature flags

The default features are `cron` and `serde`.

| Feature | Provides | Enabled by default |
| --- | --- | :---: |
| `cron` | `cron::Schedule` adapter; enables `chrono`. | Yes |
| `serde` | Serialization for `Tick` and `Config`. | Yes |
| `chrono` | Chrono and chrono-tz timezone support. | Via `cron` |
| `english` | `EnglishRoutine` for English scheduling expressions; enables `chrono`. | No |
| `builder` | Fluent schedule construction; enables `chrono`. See the UTC and interval limitations below. | No |
| `jiff` | `JiffSchedule` and jiff timezone support. | No |
| `docsrs` | Generated feature reference in the API documentation. | No |

With `default-features = false`, the scheduler and custom `Schedule<Tz>` implementations remain available. Without `chrono`, `timezone::Utc` is a lightweight marker.

## Examples

Run these commands from the repository root:

| Example | Demonstrates | Command |
| --- | --- | --- |
| [basic](examples/basic.rs) | Cron expressions, retries, and shared data. | `cargo run --locked --example basic` |
| [builder](examples/builder.rs) | Fluent schedules and Worker execution. | `cargo run --locked --example builder --features builder` |
| [timezone](examples/timezone.rs) | Timezone-typed ticks with the builder. | `cargo run --locked --example timezone --features builder` |
| [english](examples/english.rs) | English routines with local time and retries. | `cargo run --locked --example english --features english` |
| [config](examples/config.rs) | Serialized Worker, backend, and middleware configuration. | `cargo run --locked --example config --features english,serde` |
| [custom](examples/custom.rs) | A custom local-time daily schedule. | `cargo run --locked --example custom --features chrono` |

The examples schedule recurring work and normally continue until interrupted. `basic` uses `@daily`, so it does not produce an immediate tick. The `custom` example currently requests `Data<usize>` without injecting it into the Worker; its handler fails argument extraction when a tick arrives.

## How scheduling works

`Schedule<Tz>` calculates the next occurrence. `CronScheduler` caches that tick and waits until it is due. When polled after the timer completes, it emits a task with a timestamp-derived ULID and prefetches the following tick. The Worker then runs the handler through its configured middleware.

| Type | Responsibility |
| --- | --- |
| `Schedule<Tz>` | Return the next `Tick<Tz>`, or `None` when the schedule is exhausted. |
| `Tick<Tz>` | Hold a UTC UNIX timestamp in seconds; `Tz` describes the schedule's timezone type. |
| `CronScheduler<S, Tz>` | Adapt a schedule into a miju task source or a stream of ticks. |
| `Config<S, Tz>` | Hold the schedule and timezone; serialize them as strings when Serde is enabled. |

Cron and English adapters calculate upcoming occurrences from the current clock. The builder retains its previous tick. The timer is runtime-independent: Tokio's paused clock does not advance it. Tick serialization contains `timestamp` and `_marker`.

### Persistence and composition

`CronScheduler` is an ephemeral backend. Use `pipe_to` to forward emitted tasks to another backend, as shown in [the file-backend integration test](tests/backend.rs). Only tasks already delivered to that backend can benefit from its storage guarantees.

The scheduler does not persist schedule rules, replay missed occurrences after downtime, elect a leader, or guarantee exactly-once execution. SQLite persistence and workflow examples are not included in this repository.

### Errors and lifecycle

- **Invalid timestamps**: a tick outside the platform's `SystemTime` range returns `InvalidTimestamp` when polled. It remains cached, so repeated polling returns the same error without advancing the schedule.
- **Expired ticks**: a tick already in the past before timer creation returns `OutOfRange` and remains cached. A timer that was created earlier can still yield its task after the scheduled time.
- **Close and clone**: `poll_close` acknowledges immediately without resetting the scheduler. Dropping the scheduler cancels its timer. Cloning or replacing the timezone clears the cached tick and timer while preserving the schedule's current state.
- **Worker EOF with miju-core 0.1.0**: a finite tick stream ends, but a Worker reaching that end does not automatically complete shutdown. Explicit Worker stop works. The repository still depends on this published version, so the automatic-completion regression test remains failing.

### Builder limitations

The current builder calculates in UTC even when another timezone is supplied. Without an `at` time, `every(n).minutes()` advances by `n * n` minutes; with `at`, it advances to the next matching minute. A weekday without `.week()` has no unit and produces no tick. Zero intervals do not advance, and sufficiently large minute intervals overflow the intermediate `u32` multiplication in debug builds.

## API documentation

Build the API reference locally:

```sh
cargo doc --locked --all-features --no-deps --open
```

See [miju](https://github.com/nanfeng-labs/miju) for Worker lifecycle, middleware, and task-processing concepts.
