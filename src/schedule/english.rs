use std::{borrow::Cow, str::FromStr};

use english_to_cron::Cron;

use crate::{Tick, schedule::Schedule};

/// Represents an English routine for scheduling tasks.
#[derive(Debug, Clone)]
pub struct EnglishRoutine {
    schedule: cron::Schedule,
    input: String,
}

impl EnglishRoutine {
    /// Creates a new `EnglishRoutine` from the given input string.
    ///
    /// Parsing is case-insensitive. Multiple clock times with different minutes
    /// are rejected because the upstream parser cannot represent them accurately.
    pub fn new<S: Into<Cow<'static, str>>>(s: S) -> Result<Self, EnglishRoutineError> {
        Self::try_from(s.into())
    }

    /// Returns the original input string used to create the `EnglishRoutine`.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Returns a reference to the `cron::Schedule` used by the `EnglishRoutine`.
    pub fn schedule(&self) -> &cron::Schedule {
        &self.schedule
    }
}

impl FromStr for EnglishRoutine {
    type Err = EnglishRoutineError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EnglishRoutine::try_from(Cow::Borrowed(s))
    }
}

impl TryFrom<String> for EnglishRoutine {
    type Error = EnglishRoutineError;

    fn try_from(expression: String) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Owned(expression))
    }
}

impl TryFrom<&str> for EnglishRoutine {
    type Error = EnglishRoutineError;

    fn try_from(expression: &str) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Borrowed(expression))
    }
}

impl TryFrom<Cow<'_, str>> for EnglishRoutine {
    type Error = EnglishRoutineError;

    fn try_from(s: Cow<'_, str>) -> Result<Self, Self::Error> {
        let normalized = s.to_ascii_lowercase();
        let cron = Cron::from_str(&normalized)?;
        if has_conflicting_clock_minutes(&normalized) {
            return Err(english_to_cron::Error::IncorrectValue {
                state: "clock_time".into(),
                error: "Clock times with different minutes require separate routines".into(),
            }
            .into());
        }
        Ok(EnglishRoutine {
            schedule: TryFrom::try_from(cron.to_string())?,
            input: s.to_string(),
        })
    }
}

fn has_conflicting_clock_minutes(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut pos = 0;
    let mut minute = None;
    while pos < bytes.len() {
        let current = if bytes[pos].is_ascii_digit() {
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if bytes.get(pos) == Some(&b':') {
                pos += 1;
                let start = pos;
                while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                input[start..pos].parse::<u32>().ok()
            } else {
                let suffix = bytes[pos..].strip_prefix(b" ").unwrap_or(&bytes[pos..]);
                (suffix.starts_with(b"am") || suffix.starts_with(b"pm")).then_some(0)
            }
        } else if bytes[pos..].starts_with(b"noon") {
            pos += 4;
            Some(0)
        } else if bytes[pos..].starts_with(b"midnight") {
            pos += 8;
            Some(0)
        } else {
            pos += 1;
            continue;
        };
        // The upstream parser retains only one minute field across clock times.
        if let Some(current) = current {
            if minute.is_some_and(|previous| previous != current) {
                return true;
            }
            minute = Some(current);
        }
    }
    false
}

/// Errors that can occur while parsing or using an English routine
#[derive(Debug)]
pub enum EnglishRoutineError {
    /// Error related to the cron expression
    InvalidStatement(english_to_cron::Error),
    /// Error related to the schedule
    ScheduleError(cron::error::Error),
}

impl From<english_to_cron::Error> for EnglishRoutineError {
    fn from(err: english_to_cron::Error) -> Self {
        EnglishRoutineError::InvalidStatement(err)
    }
}

impl std::error::Error for EnglishRoutineError {}

impl std::fmt::Display for EnglishRoutineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnglishRoutineError::InvalidStatement(err) => write!(f, "Invalid statement: {}", err),
            EnglishRoutineError::ScheduleError(err) => write!(f, "Schedule error: {}", err),
        }
    }
}

impl From<cron::error::Error> for EnglishRoutineError {
    fn from(e: cron::error::Error) -> Self {
        EnglishRoutineError::ScheduleError(e)
    }
}

impl<Tz: chrono::TimeZone> Schedule<Tz> for EnglishRoutine {
    fn next_tick(&mut self, timezone: &Tz) -> Option<Tick<Tz>> {
        self.schedule
            .upcoming(timezone.clone())
            .next()
            .map(|s| s.timestamp() as _)
            .map(Tick::new)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use miju_core::{
        error::BoxDynError,
        task::task_id::TaskId,
        worker::{builder::WorkerBuilder, event::Event, ext::event_listener::EventListenerExt},
    };

    use crate::{backend::CronScheduler, tick::Tick};

    use super::*;

    #[tokio::test]
    async fn basic_worker() {
        let schedule = EnglishRoutine::from_str("every second").unwrap();
        let backend = CronScheduler::new(schedule);

        async fn send_reminder(job: Tick, id: TaskId) -> Result<(), BoxDynError> {
            println!("Running cronjob for timestamp: {:?} with id {}", job, id);
            tokio::time::sleep(Duration::from_secs(1)).await;
            Err("All failing".into())
        }

        let worker = WorkerBuilder::new("rango-tango")
            .backend(backend)
            .on_event(move |ctx, ev| {
                println!("{:?}", ev);
                let ctx = ctx.clone();
                if matches!(ev, Event::Error(_)) {
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        ctx.stop().unwrap();
                    });
                }
            })
            .build(send_reminder);
        worker.run().await.unwrap();
    }
}
