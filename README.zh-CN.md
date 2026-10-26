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

## 特性

- **多种调度方式**：支持 cron 表达式、英文规则、链式 builder 和 jiff 调度。
- **类型化 tick 与时区**：处理器接收 `Tick<Tz>`，可按适配器能力使用 Chrono 或 jiff 的时区支持。
- **自定义调度**：实现 `Schedule<Tz>`，自行决定下一个任务的执行时间。
- **Worker 集成**：与 miju 的共享数据、重试和中间件组合使用。
- **直接消费 tick 流**：通过 `into_stream()` 获取调度事件，无需创建 Worker。
- **可选序列化**：使用 Serde 编解码调度配置和 tick。

## 安装

使用本仓库当前源码，并搭配已发布的 miju 依赖：

```toml
[dependencies]
miju-cron = { git = "https://github.com/nanfeng-labs/miju-cron.git", branch = "main" }
miju = "0.1.0"
cron = "0.17"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

快速开始使用了 `cron::Schedule`，因此应用需要显式添加 `cron` 依赖。请提交应用的 `Cargo.lock`，以固定本次解析到的 Git 提交。

最低支持的 Rust 版本为 **1.89.0**。CI 使用 Rust 1.89.0 检查完整 feature 集。

## 快速开始

使用六字段 cron 表达式，每十秒调用一次处理器。表达式的第一个字段表示秒：

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

这是一个持续运行的周期 Worker，`run()` 会等待 Worker 停止。可中断进程结束这个最小示例；应用也可以通过 `WorkerContext::stop()` 请求关闭。

重试和处理器共享数据的用法见 [`examples/basic.rs`](examples/basic.rs)。

## Feature 开关

默认启用 `cron` 和 `serde`。

| Feature | 提供的能力 | 默认启用 |
| --- | --- | :---: |
| `cron` | `cron::Schedule` 适配器，同时启用 `chrono`。 | 是 |
| `serde` | `Tick` 和 `Config` 的序列化。 | 是 |
| `chrono` | Chrono 和 chrono-tz 时区支持。 | 由 `cron` 启用 |
| `english` | 使用英文调度规则的 `EnglishRoutine`，同时启用 `chrono`。 | 否 |
| `builder` | 链式构建调度规则，同时启用 `chrono`；调度及夏令时行为见下文。 | 否 |
| `jiff` | `JiffSchedule` 和 jiff 时区支持。 | 否 |
| `docsrs` | 在 API 文档中生成 feature 参考。 | 否 |

设置 `default-features = false` 后，调度器和自定义 `Schedule<Tz>` 实现仍可使用。未启用 `chrono` 时，`timezone::Utc` 是一个轻量标记类型。

## 示例

在仓库根目录运行：

| 示例 | 内容 | 命令 |
| --- | --- | --- |
| [basic](examples/basic.rs) | Cron 表达式、重试和共享数据。 | `cargo run --locked --example basic` |
| [builder](examples/builder.rs) | 链式构建调度规则并运行 Worker。 | `cargo run --locked --example builder --features builder` |
| [timezone](examples/timezone.rs) | 带时区类型的 tick 与 builder。 | `cargo run --locked --example timezone --features builder` |
| [english](examples/english.rs) | 英文规则、本地时区和重试。 | `cargo run --locked --example english --features english` |
| [config](examples/config.rs) | Worker、后端和中间件的配置序列化。 | `cargo run --locked --example config --features english,serde` |
| [custom](examples/custom.rs) | 自定义本地时间的每日调度。 | `cargo run --locked --example custom --features chrono` |

这些示例会持续调度任务，通常需要中断进程才能退出。`basic` 使用 `@daily`，不会立即产生 tick。`custom` 示例目前请求 `Data<usize>`，但没有向 Worker 注入该数据，因此 tick 到达后，处理器会因参数提取失败而报错。

## 调度如何运行

`Schedule<Tz>` 计算下一次触发时间，`CronScheduler` 缓存该 tick 并等待定时器到期。定时器完成后，再次轮询调度器会产生一个任务，其 ULID 根据计划时间生成，同时预取下一个 tick。Worker 随后通过配置的中间件调用处理器。

| 类型 | 职责 |
| --- | --- |
| `Schedule<Tz>` | 返回下一个 `Tick<Tz>`；调度结束时返回 `None`。 |
| `Tick<Tz>` | 保存以秒为单位的 UTC UNIX 时间戳；`Tz` 表示调度使用的时区类型。 |
| `CronScheduler<S, Tz>` | 将调度规则适配为 miju 任务来源或 tick 流。 |
| `Config<S, Tz>` | 保存调度规则与时区；启用 Serde 后将两者序列化为字符串。 |

Cron 和英文适配器基于当前时钟计算后续时间，builder 则保留上一次 tick。定时器独立于异步运行时，暂停 Tokio 的时钟不会推进它。Tick 的序列化字段为 `timestamp` 和 `_marker`。

### 持久化与组合

`CronScheduler` 是临时后端。可以使用 `pipe_to` 将已产生的任务转交其他后端，示例见[文件后端集成测试](tests/backend.rs)。只有已经送达该后端的任务，才能获得它提供的存储保证。

调度器不持久化调度规则，不补发停机期间错过的事件，不选举主节点，也不保证恰好执行一次。本仓库尚未包含 SQLite 持久化和工作流示例。

### 错误与生命周期

- **无效时间戳**：tick 超出平台 `SystemTime` 的表示范围时，轮询返回 `InvalidTimestamp`。该 tick 仍保留在缓存中，重复轮询返回相同错误，不推进调度规则。
- **过期 tick**：创建定时器之前，若 tick 已在过去，返回 `OutOfRange` 并保留缓存。若定时器已经创建，即使晚于计划时间再次轮询，仍可取得对应任务。
- **关闭与克隆**：`poll_close` 立即确认关闭，但不重置调度器。丢弃调度器会取消其定时器；克隆或替换时区会清除缓存 tick 和定时器，同时保留调度规则的当前状态。
- **miju-core 0.1.0 的 Worker EOF 问题**：有限 tick 流能够结束，但 Worker 读到流末尾后不会自动完成关闭。显式停止 Worker 可以正常工作。本仓库仍依赖这一发布版本，因此自动关闭的回归测试仍会失败。

### Builder 调度行为

Builder 使用传给 `next_tick` 的时区，包括通过 `CronScheduler::with_timezone` 指定的时区。`every(n).minutes()` 每次推进 `n` 分钟；配合 `.at(":17")` 时，首次 tick 对齐到即将到来的第 17 秒，之后每次推进 `n` 个本地分钟。没有 `at` 的分钟、小时规则按实际经过时间推进；日、周规则在时区偏移变化时保留本地钟表时间。

`.monday()` 等星期方法会自动选择周调度，替换此前的时间单位并保留间隔，因此 `.each().monday()` 不再需要先调用 `.week()`。

零间隔、无效 `at` 字符串、日期溢出，以及夏令时切换中重复或不存在的本地时间都会返回 `None`。Builder 不会擅自选择重复时间中的一次，也不会将不存在的时间向后挪动。对 Worker 而言，`None` 表示调度结束；若需要不同的夏令时处理方式，可使用 cron 适配器。`at` 支持 `:SS`、`HH:MM` 和 `HH:MM:SS`：分钟规则使用秒，小时规则使用分和秒，日、周规则使用完整时间。

## API 文档

在本地构建并打开 API 参考：

```sh
cargo doc --locked --all-features --no-deps --open
```

Worker 生命周期、中间件和任务处理概念可参阅 [miju 主仓库](https://github.com/nanfeng-labs/miju)。
