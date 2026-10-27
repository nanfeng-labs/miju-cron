<p align="center">
  <img src="assets/miju-cron-header.svg" alt="miju-cron" width="480" />
</p>

<h1 align="center">miju-cron</h1>

<div align="center">
  <strong>面向 Rust 的周期任务调度，与 miju Worker 组合使用</strong>
</div>

<br />

<div align="center">
  <a href="https://crates.io/crates/miju-cron">
    <img src="https://img.shields.io/crates/v/miju-cron.svg" alt="Crates.io 版本" />
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
  <a href="README.md">English</a>
</div>


miju-cron 将周期调度规则转换为 miju Worker 可以处理的类型化任务，适用于定期报表、提醒和维护任务，也可以直接以流的形式消费调度事件。

## 特性

- **多种调度格式**：支持 cron 表达式、英文规则、链式 builder 和 Jiff 调度。
- **时区支持**：根据适配器选择 UTC、本地时间或具名时区。
- **Worker 中间件**：组合 miju 的重试、共享数据和执行限制。
- **后端组合**：通过 `pipe_to` 将产生的任务转交存储后端。
- **自定义规则**：实现 `Schedule<Tz>`，定义自己的时间规则或有限事件序列。
- **配置序列化**：使用 Serde 序列化 tick 和支持该能力的调度配置。

## 安装

下面的依赖用于后文的 cron Worker 示例。需要 Rust **1.89.0** 或更高版本。

```toml
[dependencies]
miju-cron = "0.1.1"
miju = "0.1.1"
cron = "0.17"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

示例直接导入 `cron::Schedule`，因此需要显式添加 `cron`。miju-cron 默认启用 `cron` 和 `serde`。

## 用法

### Cron 表达式与中间件

这个 Worker 每十秒处理一个 tick。六字段 cron 表达式以秒开头；处理器的 `Data<String>` 参数接收通过 `.data(...)` 注册的值。

```rust,no_run
# #[cfg(feature = "cron")]
# mod example {
use cron::Schedule;
use miju::{layers::retry::RetryPolicy, prelude::*};
use miju_cron::{CronScheduler, Tick};
use std::str::FromStr;

async fn report(tick: Tick, name: Data<String>) {
    println!("{}: scheduled at {}", name.as_str(), tick.get_timestamp());
}

#[tokio::main]
async fn main() -> Result<(), BoxDynError> {
    let schedule = Schedule::from_str("*/10 * * * * *")?;
    WorkerBuilder::new("periodic-report")
        .backend(CronScheduler::new(schedule))
        .retry(RetryPolicy::retries(3))
        .data(String::from("daily-summary"))
        .build(report)
        .run()
        .await?;
    Ok(())
}
# }
```

`run()` 会等待 Worker 停止。可中断示例进程退出，也可以在应用中调用 `WorkerContext::stop()`。重试策略在处理器返回错误时生效；上面的简单处理器始终成功。更多共享数据用法见 [basic](examples/basic.rs)。

### 链式调度与时区

通过 `miju-cron = { version = "0.1.1", features = ["builder"] }` 启用 builder。运行此示例还需添加 `chrono = "0.4"` 和 `chrono-tz = "0.10"`。

```rust
# #[cfg(feature = "builder")]
# {
use miju_cron::{CronScheduler, builder::schedule};

let every_five_minutes = schedule().every(5).minutes().build::<chrono::Utc>();
let monday_morning = schedule().each().monday().at("09:30").build::<chrono_tz::Tz>();
let backend = CronScheduler::new(monday_morning)
    .with_timezone(chrono_tz::Asia::Shanghai);
# }
```

将 `backend` 传给 Worker builder 的 `.backend(...)`，处理器接收 `Tick<chrono_tz::Tz>`。`every(5).minutes()` 每次推进五分钟，`.monday()` 等星期方法会自动选择周调度。使用本地时间的完整 Worker 示例见 [timezone](examples/timezone.rs)。

未指定 `.at(...)` 的分钟、小时规则按实际经过时间推进，日、周规则保留本地钟表时间。零间隔、无效时间、日期溢出，以及夏令时切换中重复或不存在的本地时间会使 builder 返回 `None`，结束调度；请根据业务选择合适的 DST 策略。`at` 支持 `:SS`、`HH:MM`、`HH:MM:SS`，使用所选时间单位对应的字段。

### 英文规则

启用 `english` feature，并添加 `chrono = "0.4"` 以使用本地时间：

```rust
# #[cfg(feature = "english")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use miju_cron::{CronScheduler, english::EnglishRoutine};

let routine = EnglishRoutine::new("every 5 minutes")?;
let backend = CronScheduler::new(routine).with_timezone(chrono::Local);
# Ok(())
# }
# #[cfg(not(feature = "english"))]
# fn main() {}
```

解析失败会返回错误。英文规则转换为 cron 表达式，并按当前时钟计算下次执行时间。Worker 集成见 [english](examples/english.rs)；调度、时区和中间件的序列化配置见 [config](examples/config.rs)。

### 将 tick 转交其他后端

调度器本身是临时后端，`pipe_to` 可以将产生的任务交给其他后端接收。运行此示例需添加 `miju-core = "0.1.1"` 和 `miju-file-storage = "0.1.1"`：

```rust,no_run
# #[cfg(all(feature = "cron", feature = "serde"))]
# mod example {
use cron::Schedule;
use miju::prelude::*;
use miju_core::backend::ext::BackendExt;
use miju_cron::{CronScheduler, Tick};
use miju_file_storage::JsonStorage;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), BoxDynError> {
    let schedule = Schedule::from_str("*/10 * * * * *")?;
    let storage = JsonStorage::new_temp()?;
    let backend = CronScheduler::new(schedule).pipe_to(storage);
    WorkerBuilder::new("file-backed-report")
        .backend(backend)
        .build(async |tick: Tick| {
            println!("Received {}", tick.get_timestamp());
        })
        .run()
        .await?;
    Ok(())
}
# }
```

`new_temp()` 创建的是演示用临时存储。只有已送达目标后端的任务才能获得其存储保证；用于应用前，请阅读文件后端的持久化和并发约定。这种组合不会持久化调度规则、补发停机期间的任务、选举主节点或保证恰好执行一次。

### 自定义调度

实现 `Schedule<Tz>`，返回以秒为单位的下一个 UTC UNIX 时间戳；没有后续事件时返回 `None`。下面的规则产生两个 tick：

```rust
use miju_cron::{CronScheduler, Schedule, Tick, timezone::Utc};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct Occurrences(VecDeque<u64>);

impl Schedule<Utc> for Occurrences {
    fn next_tick(&mut self, _: &Utc) -> Option<Tick<Utc>> {
        self.0.pop_front().map(Tick::new)
    }
}

let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
let schedule = Occurrences(VecDeque::from([now + 10, now + 20]));
let backend = CronScheduler::new(schedule);
# Ok::<(), Box<dyn std::error::Error>>(())
```

使用 miju 0.1.1 时，Worker 可以在有限调度流耗尽后完成关闭。自定义实现应返回有效的时间戳，并自行定义时区切换和错过执行时间时的处理方式。

### 直接消费 tick 流

添加 `futures-util = "0.3"`，即可在不创建 Worker 的情况下消费 tick：

```rust,no_run
# #[cfg(feature = "cron")]
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
use cron::Schedule;
use futures_util::{StreamExt, pin_mut};
use miju_cron::CronScheduler;
use std::str::FromStr;

let ticks = CronScheduler::new(Schedule::from_str("*/10 * * * * *")?).into_stream();
pin_mut!(ticks);
while let Some(tick) = ticks.next().await {
    println!("Scheduled at {}", tick?.get_timestamp());
}
# Ok(())
# }
```

## Feature flags

| Feature | 能力 | 默认启用 |
| --- | --- | :---: |
| `cron` | `cron::Schedule` 适配器，同时启用 `chrono`。 | 是 |
| `serde` | `Tick` 和 `Config` 的序列化。 | 是 |
| `chrono` | Chrono 和 chrono-tz 时区支持。 | 由 `cron` 启用 |
| `builder` | 链式构建调度规则，同时启用 `chrono`。 | 否 |
| `english` | 英文调度规则，同时启用 `chrono`。 | 否 |
| `jiff` | `JiffSchedule` 和 Jiff 时区支持。 | 否 |
| `docsrs` | API 文档中的自动 feature 参考。 | 否 |

设置 `default-features = false` 后，仍可使用调度器和自定义规则。不启用 `chrono` 时，`timezone::Utc` 是轻量标记类型。

## 调度行为

`CronScheduler` 缓存一个 tick，等待到期后产生任务，并预取下一个执行时间。任务使用由时间戳生成的 ULID。Cron 和英文适配器依据当前时钟计算，builder 则从前一个 tick 继续推进。定时器依赖真实经过时间，不受 Tokio 暂停时钟驱动。

- `Tick<Tz>` 保存以秒为单位的 UTC UNIX 时间戳；时区类型描述调度规则如何解释时间。
- `Config<S, Tz>` 保存调度规则和时区，Serde 将二者存为字符串；tick 的序列化字段为 `timestamp` 和 `_marker`。
- 无法表示的时间戳返回 `InvalidTimestamp` 并保留缓存，重复轮询仍返回错误，不推进规则。有效但已到期的 tick 会延迟交付，保留原时间戳和任务 ID；消费者背压导致预取 tick 过期时也遵循此规则。这不会补建调度适配器未返回的执行时间。
- 在截止时间前创建的定时器可以在较晚的轮询中产生任务。克隆或切换时区会清除定时器和缓存 tick，但保留调度规则已经推进的状态。
- `poll_close` 立即确认关闭，不重置调度器；丢弃调度器会取消定时器。

## 示例与文档

在仓库根目录运行：

| 示例 | 内容 | 命令 |
| --- | --- | --- |
| [basic](examples/basic.rs) | Cron 表达式、重试和共享数据。 | `cargo run --locked --example basic` |
| [builder](examples/builder.rs) | 链式构建调度规则并运行 Worker。 | `cargo run --locked --example builder --features builder` |
| [timezone](examples/timezone.rs) | 带时区类型的 tick 与 builder。 | `cargo run --locked --example timezone --features builder` |
| [english](examples/english.rs) | 英文规则、本地时区和重试。 | `cargo run --locked --example english --features english` |
| [config](examples/config.rs) | Worker、后端和中间件的配置序列化。 | `cargo run --locked --example config --features english,serde` |
| [custom](examples/custom.rs) | 自定义本地时间的每日调度。 | `cargo run --locked --example custom --features chrono` |

这些示例会持续调度任务，通常需要中断进程才能退出。`basic` 使用 `@daily`，不会立即产生 tick。

在本地构建 API 文档：

```sh
cargo doc --locked --all-features --no-deps --open
```

Worker 生命周期与中间件请参阅 [miju](https://github.com/nanfeng-labs/miju)。

## 许可证

本项目采用 [MIT 许可证](https://github.com/nanfeng-labs/miju-cron/blob/main/LICENSE)。
