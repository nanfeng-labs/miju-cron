use std::{
    marker::PhantomData,
    time::{Duration, SystemTime},
};

use crate::timezone::Utc;

/// Represents a single tick in the cron schedule
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tick<Tz = Utc> {
    /// The timestamp of the tick in UTC
    timestamp: u64,
    _marker: PhantomData<Tz>,
}

impl<Tz> Tick<Tz> {
    /// Create a new context provided a timestamp
    pub fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            _marker: PhantomData,
        }
    }

    /// Get the inner timestamp
    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the duration between this tick and a [`SystemTime`].
    ///
    /// The `Ok` value is returned when the tick is at or after `system_time`,
    /// while the `Err` value is returned when the tick is before `system_time`.
    pub(crate) fn signed_duration_since(&self, time: SystemTime) -> Result<Duration, Duration> {
        self.system_time()
            .duration_since(time)
            .map_err(|e| e.duration())
    }

    pub(crate) fn system_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(self.timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_is_seconds_and_signed_delta_preserves_subseconds() {
        let tick = Tick::<Utc>::new(42);
        assert_eq!(
            tick.system_time(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(42)
        );
        assert_eq!(
            tick.signed_duration_since(tick.system_time()),
            Ok(Duration::ZERO)
        );
        assert_eq!(
            tick.signed_duration_since(tick.system_time() - Duration::from_millis(1)),
            Ok(Duration::from_millis(1))
        );
        assert_eq!(
            tick.signed_duration_since(tick.system_time() + Duration::from_millis(1)),
            Err(Duration::from_millis(1))
        );
    }
}
