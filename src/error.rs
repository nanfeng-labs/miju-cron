use std::time::Duration;

/// Represents an error emitted by `CronScheduler` polling
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The tick timestamp cannot be represented by the platform's system clock.
    #[error("Tick timestamp {tick} cannot be represented as SystemTime")]
    InvalidTimestamp {
        /// The unsupported UNIX timestamp in seconds.
        tick: u64,
    },
    /// The tick precedes the reference time used to calculate its duration.
    /// Scheduler polling treats this as an already-due tick and delivers it late.
    #[error("Tick out of range for {tick:?} (past duration:  {duration:?})")]
    OutOfRange {
        /// The past duration
        duration: Duration,
        /// The missed tick
        tick: u64,
    },
}
