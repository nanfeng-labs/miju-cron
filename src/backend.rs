use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, SystemTime},
};

use futures_util::Stream;
use miju_core::{
    backend::{Backend, BackendConfig, TryNewBackend, finalize::Ephemeral},
    features_table,
    layers::Identity,
    task::{Task, builder::TaskBuilder, task_id::TaskId},
    timer::Delay,
    worker::context::WorkerContext,
};
use ulid::Ulid;

use crate::{config::Config, error::Error, schedule::Schedule, tick::Tick, timezone::Utc};

/// What to do when more than one tick is already due, for example after
/// consumer backpressure or when resuming from an old cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MissedTickPolicy {
    /// Deliver the earliest due tick, then continue with the first tick after now.
    #[default]
    Once,
    /// Deliver every due tick in order.
    CatchUp,
}

/// A backend that produces tasks based on a cron schedule.
#[doc = features_table! {
    setup = "unreachable!();",
    TaskSink => not_supported("You cannot push tasks to a cron scheduler"),
}]
#[derive(Debug)]
pub struct CronScheduler<S, Tz = Utc> {
    config: Config<S, Tz>,
    name: Option<Arc<str>>,
    missed: MissedTickPolicy,
    /// Timestamp of the last delivered tick, or the resume point.
    cursor: Option<u64>,
    next_tick: Option<Tick<Tz>>,
    delay: Option<Delay>,
}

impl<S> CronScheduler<S, Utc> {
    /// Build a new cron scheduler from a [Schedule] using the [Utc] timezone
    pub const fn new(schedule: S) -> CronScheduler<S> {
        CronScheduler::from_config(Config::new(schedule, Utc))
    }
}

impl<S, Tz> CronScheduler<S, Tz> {
    const fn from_config(config: Config<S, Tz>) -> Self {
        CronScheduler {
            config,
            name: None,
            missed: MissedTickPolicy::Once,
            cursor: None,
            next_tick: None,
            delay: None,
        }
    }

    /// Name the scheduler so that each tick gets a deterministic task ID and
    /// idempotency key (`"{name}@{timestamp}"`).
    ///
    /// Replicas or restarts that share a name produce identical IDs for the same
    /// tick, so sinks that honour idempotency keys store it once, and miju
    /// rejects a duplicate that is still running. Give schedulers that feed the
    /// same worker or storage distinct names.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(Arc::from(name.into()));
        self
    }

    /// Choose what happens when several ticks are already due.
    #[must_use]
    pub fn missed_ticks(mut self, policy: MissedTickPolicy) -> Self {
        self.missed = policy;
        self
    }

    /// Start after `tick` instead of after the current time.
    ///
    /// Pass the last tick a previous run handled to continue where it stopped;
    /// ticks due in between are delivered according to [`MissedTickPolicy`].
    #[must_use]
    pub fn resume_after<T>(mut self, tick: &Tick<T>) -> Self {
        self.cursor = Some(tick.get_timestamp());
        self.next_tick = None;
        self.delay = None;
        self
    }

    /// Build a new cron with specific timezone
    pub fn with_timezone<T>(self, timezone: T) -> CronScheduler<S, T> {
        CronScheduler {
            config: Config::new(self.config.schedule, timezone),
            name: self.name,
            missed: self.missed,
            cursor: self.cursor,
            next_tick: None,
            delay: None,
        }
    }
}

impl<Schedule: Clone, Timezone: Clone> Clone for CronScheduler<Schedule, Timezone> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            name: self.name.clone(),
            missed: self.missed,
            cursor: self.cursor,
            next_tick: None,
            delay: None,
        }
    }
}

impl<S: Schedule<Tz>, Tz> CronScheduler<S, Tz> {
    /// Convert the cron scheduler into a stream of ticks
    pub fn into_stream(self) -> impl Stream<Item = Result<Tick<Tz>, Error>>
    where
        Self: Backend<Task = Task<Tick<Tz>>, Error = Error> + BackendConfig<Args = Tick<Tz>>,
    {
        let mut cron = self;
        let worker = WorkerContext::new("cron-scheduler");
        futures_util::stream::poll_fn(move |cx| {
            cron.poll_next(cx, &worker)
                .map(|next| next.map(|task| task.map(|task| task.args)))
        })
    }
}

fn unix_secs(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// FNV-1a: stable across Rust versions, unlike `DefaultHasher`.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

impl<S, Tz> Backend for CronScheduler<S, Tz>
where
    S: Schedule<Tz>,
    Tz: Debug + Clone,
{
    type Task = Task<Tick<Tz>>;
    type Error = Error;

    fn poll_ready(
        &mut self,
        _: &mut Context<'_>,
        _: &WorkerContext,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        _: &WorkerContext,
    ) -> Poll<Option<Result<Self::Task, Self::Error>>> {
        tracing::trace!(
            has_next_tick = self.next_tick.is_some(),
            has_delay = self.delay.is_some(),
            "polling cron scheduler"
        );

        let tz = self.config.timezone().clone();
        let now = loop {
            if self.next_tick.is_none() {
                let after = self.cursor.unwrap_or_else(|| unix_secs(SystemTime::now()));
                tracing::trace!(after, "calculating next cron tick");
                self.next_tick = self.config.schedule.next_after(&Tick::new(after), &tz);
                if self.next_tick.is_none() {
                    tracing::debug!("cron schedule exhausted");
                    return Poll::Ready(None);
                }
            }

            let next = self.next_tick.as_ref().unwrap();
            let now = SystemTime::now();
            let remaining = match next.signed_duration_since(now) {
                Ok(d) => d,
                // Overdue (backpressure or resume): deliver with its original timestamp.
                Err(Error::OutOfRange { .. }) => Duration::ZERO,
                Err(e) => {
                    tracing::warn!(tick = ?next, error = ?e, "cron tick is out of range");
                    return Poll::Ready(Some(Err(e)));
                }
            };
            if remaining.is_zero() {
                break now;
            }

            let delay = self.delay.get_or_insert_with(|| {
                tracing::trace!(tick = ?next, ?remaining, "creating delay until next cron tick");
                Delay::new(remaining)
            });
            match Pin::new(delay).poll(cx) {
                Poll::Pending => return Poll::Pending,
                // The timer is monotonic; re-check the wall clock before firing.
                Poll::Ready(()) => self.delay = None,
            }
        };

        self.delay = None;
        let fired = self.next_tick.take().unwrap();
        let timestamp = fired.get_timestamp();
        let cursor = match self.missed {
            MissedTickPolicy::CatchUp => timestamp,
            MissedTickPolicy::Once => timestamp.max(unix_secs(now)),
        };
        self.cursor = Some(cursor);
        tracing::debug!(tick = ?fired, cursor, "cron tick fired");

        self.next_tick = self.config.schedule.next_after(&Tick::new(cursor), &tz);
        tracing::trace!(next_tick = ?self.next_tick, "scheduled next cron tick");

        let time = match fired.system_time() {
            Ok(time) => time,
            Err(e) => return Poll::Ready(Some(Err(e))),
        };
        let builder = match &self.name {
            Some(name) => {
                // Same bits as `Ulid::from_datetime`'s timestamp, with a name-derived random part.
                let millis = (u128::from(timestamp) * 1000) as u64;
                TaskBuilder::new(fired)
                    .task_id(TaskId::Ulid(Ulid::from_parts(
                        millis,
                        u128::from(fnv1a(name.as_bytes())),
                    )))
                    .idempotency_key(format!("{name}@{timestamp}"))
            }
            None => TaskBuilder::new(fired).task_id(TaskId::Ulid(Ulid::from_datetime(time))),
        };
        Poll::Ready(Some(Ok(builder.build())))
    }

    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        _: &WorkerContext,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl<S, Tz> BackendConfig for CronScheduler<S, Tz> {
    type Args = Tick<Tz>;
    type Config = Config<S, Tz>;
    type Kind = Ephemeral;
    type Layer = Identity;
    type Id = Ulid;

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn middleware(&mut self, _: &mut WorkerContext) -> Self::Layer {
        Identity::new()
    }
}

impl<S: Schedule<Tz>, Tz> TryNewBackend for CronScheduler<S, Tz>
where
    Self: Backend + BackendConfig<Args = Tick<Tz>, Config = Config<S, Tz>>,
{
    type Backend = Self;
    fn try_new(config: Self::Config) -> Result<Self, Self::Error> {
        Ok(Self::from_config(config))
    }
}
