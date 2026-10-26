use std::{
    marker::PhantomData,
    time::{Duration, SystemTime},
};

use crate::{Error, timezone::Utc};

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
    /// Returns an error if the tick is in the past or cannot be represented
    /// by the platform's system clock.
    pub(crate) fn signed_duration_since(&self, time: SystemTime) -> Result<Duration, Error> {
        self.system_time()?
            .duration_since(time)
            .map_err(|e| Error::OutOfRange {
                duration: e.duration(),
                tick: self.timestamp,
            })
    }

    pub(crate) fn system_time(&self) -> Result<SystemTime, Error> {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(self.timestamp))
            .ok_or(Error::InvalidTimestamp {
                tick: self.timestamp,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_is_seconds_and_signed_delta_preserves_subseconds() {
        let tick = Tick::<Utc>::new(42);
        let time = tick.system_time().unwrap();
        assert_eq!(time, SystemTime::UNIX_EPOCH + Duration::from_secs(42));
        assert_eq!(tick.signed_duration_since(time).unwrap(), Duration::ZERO);
        assert_eq!(
            tick.signed_duration_since(time - Duration::from_millis(1))
                .unwrap(),
            Duration::from_millis(1)
        );
        assert!(matches!(
            tick.signed_duration_since(time + Duration::from_millis(1)),
            Err(Error::OutOfRange { duration, tick: 42 })
                if duration == Duration::from_millis(1)
        ));
    }
}
