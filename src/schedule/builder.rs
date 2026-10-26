use std::fmt;

use chrono::{DateTime, Datelike, Duration, NaiveTime, Offset, TimeZone, Timelike, Utc, Weekday};

use crate::Tick;

/// Time units for scheduling
#[derive(Debug, Clone)]
pub enum TimeUnit {
    /// Minutes time unit
    Minutes(u32),
    /// Hours time unit
    Hours,
    /// Days time unit
    Days,
    /// Weeks time unit
    Weeks,
    /// Months time unit
    Months,
}

/// Builder for creating schedules via a fluent API
#[derive(Debug, Clone)]
pub struct ScheduleBuilder {
    interval: Option<u32>,
    unit: Option<TimeUnit>,
    weekday: Option<Weekday>,
    time: Option<String>,
}

impl Default for ScheduleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleBuilder {
    /// Creates a new `ScheduleBuilder`.
    pub fn new() -> Self {
        ScheduleBuilder {
            interval: None,
            unit: None,
            weekday: None,
            time: None,
        }
    }

    fn parse_time(&self) -> Option<NaiveTime> {
        let time = self.time.as_deref()?;
        if let Some(seconds) = time.strip_prefix(':') {
            return NaiveTime::from_hms_opt(0, 0, seconds.parse().ok()?);
        }
        let mut parts = time.split(':');
        let hour = parts.next()?.parse().ok()?;
        let minute = parts.next()?.parse().ok()?;
        let second = parts.next().map(str::parse).transpose().ok()?.unwrap_or(0);
        if parts.next().is_some() {
            return None;
        }
        NaiveTime::from_hms_opt(hour, minute, second)
    }

    fn calculate_next_execution<Tz: TimeZone>(
        &self,
        from: DateTime<Utc>,
        timezone: &Tz,
    ) -> Option<DateTime<Utc>> {
        let unit = self.unit.as_ref()?;
        let interval = self.interval.unwrap_or(match unit {
            TimeUnit::Minutes(n) => *n,
            _ => 1,
        });
        if interval == 0 {
            return None;
        }
        let interval = i64::from(interval);
        let step = match unit {
            TimeUnit::Minutes(_) => Duration::try_minutes(interval)?,
            TimeUnit::Hours => Duration::try_hours(interval)?,
            TimeUnit::Days => Duration::try_days(interval)?,
            TimeUnit::Weeks => Duration::try_weeks(interval)?,
            TimeUnit::Months => Duration::try_days(30 * interval)?,
        };
        let time = if self.time.is_some() {
            Some(self.parse_time()?)
        } else {
            None
        };

        // Minute/hour intervals without an alignment are elapsed durations.
        if time.is_none() && matches!(unit, TimeUnit::Minutes(_) | TimeUnit::Hours) {
            return from.checked_add_signed(step);
        }

        let offset = timezone.offset_from_utc_datetime(&from.naive_utc()).fix();
        let local = from.naive_utc().checked_add_offset(offset)?;
        let mut next = local;
        if let Some(weekday) = self.weekday {
            let days = (i64::from(weekday.num_days_from_monday())
                - i64::from(local.weekday().num_days_from_monday())
                + 7)
                % 7;
            next = next.checked_add_signed(Duration::days(days))?;
        }
        if let Some(time) = time {
            let time = match unit {
                TimeUnit::Minutes(_) => {
                    NaiveTime::from_hms_opt(local.hour(), local.minute(), time.second())?
                }
                TimeUnit::Hours => {
                    NaiveTime::from_hms_opt(local.hour(), time.minute(), time.second())?
                }
                _ => time,
            };
            next = next.date().and_time(time);
        }
        if next <= local {
            next = next.checked_add_signed(step)?;
        }
        // Reject DST gaps and folds instead of silently choosing another wall time.
        let candidate = timezone
            .from_local_datetime(&next)
            .single()?
            .with_timezone(&Utc);
        (candidate > from).then_some(candidate)
    }
}

/// Builder for creating intervals
#[derive(Debug, Clone)]
pub struct IntervalBuilder {
    interval: u32,
}

/// Builder for creating time units
#[derive(Debug, Clone)]
pub struct TimeUnitBuilder {
    schedule: ScheduleBuilder,
}

/// Builder for creating weekdays
#[derive(Debug, Clone)]
pub struct WeekdayBuilder {
    schedule: ScheduleBuilder,
}

impl ScheduleBuilder {
    /// Creates an interval builder. A zero interval produces no ticks.
    pub fn every(&self, interval: u32) -> IntervalBuilder {
        IntervalBuilder { interval }
    }

    /// Creates a new `TimeUnitBuilder` for specifying the time unit.
    pub fn each(&self) -> TimeUnitBuilder {
        TimeUnitBuilder {
            schedule: ScheduleBuilder::new(),
        }
    }
}

impl IntervalBuilder {
    /// Creates a new `TimeUnitBuilder` for specifying the time unit.
    pub fn minutes(self) -> TimeUnitBuilder {
        TimeUnitBuilder {
            schedule: ScheduleBuilder {
                interval: Some(self.interval),
                unit: Some(TimeUnit::Minutes(self.interval)),
                ..ScheduleBuilder::new()
            },
        }
    }
}

impl TimeUnitBuilder {
    /// Sets the time unit to hours.
    pub fn hour(mut self) -> TimeUnitBuilder {
        self.schedule.unit = Some(TimeUnit::Hours);
        self
    }

    /// Sets the time unit to days.
    pub fn day(mut self) -> TimeUnitBuilder {
        self.schedule.unit = Some(TimeUnit::Days);
        self
    }

    /// Sets the time unit to weeks.
    pub fn week(mut self) -> TimeUnitBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self
    }

    /// Selects Monday and switches the schedule to weeks, retaining the interval.
    pub fn monday(mut self) -> WeekdayBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self.schedule.weekday = Some(Weekday::Mon);
        WeekdayBuilder {
            schedule: self.schedule,
        }
    }

    /// Selects Tuesday and switches the schedule to weeks, retaining the interval.
    pub fn tuesday(mut self) -> WeekdayBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self.schedule.weekday = Some(Weekday::Tue);
        WeekdayBuilder {
            schedule: self.schedule,
        }
    }

    /// Selects Wednesday and switches the schedule to weeks, retaining the interval.
    pub fn wednesday(mut self) -> WeekdayBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self.schedule.weekday = Some(Weekday::Wed);
        WeekdayBuilder {
            schedule: self.schedule,
        }
    }

    /// Selects Thursday and switches the schedule to weeks, retaining the interval.
    pub fn thursday(mut self) -> WeekdayBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self.schedule.weekday = Some(Weekday::Thu);
        WeekdayBuilder {
            schedule: self.schedule,
        }
    }

    /// Selects Friday and switches the schedule to weeks, retaining the interval.
    pub fn friday(mut self) -> WeekdayBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self.schedule.weekday = Some(Weekday::Fri);
        WeekdayBuilder {
            schedule: self.schedule,
        }
    }

    /// Selects Saturday and switches the schedule to weeks, retaining the interval.
    pub fn saturday(mut self) -> WeekdayBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self.schedule.weekday = Some(Weekday::Sat);
        WeekdayBuilder {
            schedule: self.schedule,
        }
    }

    /// Selects Sunday and switches the schedule to weeks, retaining the interval.
    pub fn sunday(mut self) -> WeekdayBuilder {
        self.schedule.unit = Some(TimeUnit::Weeks);
        self.schedule.weekday = Some(Weekday::Sun);
        WeekdayBuilder {
            schedule: self.schedule,
        }
    }

    /// Sets the time unit to minutes.
    pub fn minute(mut self) -> TimeUnitBuilder {
        self.schedule.unit = Some(TimeUnit::Minutes(1));
        self
    }

    /// Aligns ticks to `:SS`, `HH:MM`, or `HH:MM:SS`; invalid times produce no ticks.
    pub fn at(mut self, time: &str) -> TimeUnitBuilder {
        self.schedule.time = Some(time.to_string());
        self
    }

    /// Builds the schedule iterator.
    pub fn build<Tz: TimeZone>(self) -> ScheduleIterator<Tz> {
        ScheduleIterator::new(self.schedule)
    }
}

impl WeekdayBuilder {
    /// Sets the local time for the weekday schedule; invalid times produce no ticks.
    pub fn at(mut self, time: &str) -> WeekdayBuilder {
        self.schedule.time = Some(time.to_string());
        self
    }

    /// Builds the weekday schedule.
    pub fn build<Tz: TimeZone>(self) -> ScheduleIterator<Tz> {
        ScheduleIterator::new(self.schedule)
    }
}

/// Iterator over schedule ticks in the timezone supplied to `Schedule::next_tick`.
///
/// Zero intervals, invalid `at` times, dates outside Chrono's range, and ambiguous
/// or nonexistent local times return `None`. Calendar intervals preserve local
/// wall time; minute/hour intervals without `at` use elapsed time.
#[derive(Debug, Clone)]
pub struct ScheduleIterator<Tz> {
    schedule: ScheduleBuilder,
    current: Option<Tick<Tz>>,
}

impl<Tz: chrono::TimeZone> ScheduleIterator<Tz> {
    /// Create a new schedule iterator
    pub fn new(schedule: ScheduleBuilder) -> Self {
        Self {
            schedule,
            current: None,
        }
    }
}

impl<Tz> fmt::Display for ScheduleIterator<Tz>
where
    Tz: TimeZone,
    Tz::Offset: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScheduleIterator(schedule = {:?}, ", self.schedule)?;
        match &self.current {
            Some(dt) => write!(f, "next = {})", dt.get_timestamp()),
            None => write!(f, "end)"),
        }
    }
}

/// Builder for creating schedules via a fluent API
pub fn schedule() -> ScheduleBuilder {
    ScheduleBuilder::new()
}

impl fmt::Display for ScheduleBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if let Some(interval) = self.interval {
            parts.push(format!("every {} ", interval));
        } else {
            parts.push("every ".to_string());
        }

        match &self.unit {
            Some(TimeUnit::Minutes(1)) => parts.push("minute".to_string()),
            Some(TimeUnit::Minutes(n)) => parts.push(format!("{} minutes", n)),
            Some(TimeUnit::Hours) => parts.push("hour".to_string()),
            Some(TimeUnit::Days) => parts.push("day".to_string()),
            Some(TimeUnit::Weeks) => parts.push("week".to_string()),
            Some(TimeUnit::Months) => parts.push("month".to_string()),
            None => {}
        }

        if let Some(weekday) = &self.weekday {
            parts.push(format!("{:?}", weekday).to_lowercase());
        }

        if let Some(time) = &self.time {
            parts.push(format!("at {}", time));
        }

        write!(f, "{}", parts.join(" "))
    }
}

impl<Tz: Clone + TimeZone> crate::schedule::Schedule<Tz> for ScheduleIterator<Tz> {
    fn next_tick(&mut self, timezone: &Tz) -> Option<Tick<Tz>> {
        let current = self
            .current
            .as_ref()
            .and_then(|t| i64::try_from(t.get_timestamp()).ok())
            .and_then(DateTime::from_timestamp_secs)
            .unwrap_or(Utc::now());
        let next = self
            .schedule
            .calculate_next_execution(current, timezone)
            .and_then(|dt| u64::try_from(dt.timestamp()).ok())
            .map(Tick::new);
        self.current = next.clone();
        next
    }
}

#[cfg(test)]
mod tests {
    use miju_core::{
        error::BoxDynError,
        task::task_id::TaskId,
        worker::{builder::WorkerBuilder, event::Event, ext::event_listener::EventListenerExt},
    };

    use crate::{backend::CronScheduler, tick::Tick};

    use super::*;

    use std::time::Duration;
    fn from_tick<Tz: Clone + TimeZone>(
        mut iterator: ScheduleIterator<Tz>,
        from: DateTime<Utc>,
        timezone: &Tz,
    ) -> Option<DateTime<Tz>> {
        use crate::Schedule;
        iterator.current = Some(Tick::new(from.timestamp().try_into().unwrap()));
        let tick = iterator.next_tick(timezone)?;
        timezone
            .timestamp_opt(tick.get_timestamp().try_into().unwrap(), 0)
            .single()
    }

    #[test]
    fn builder_uses_local_wall_time() {
        use chrono_tz::{America::New_York, Asia::Tokyo};
        let from = Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap();
        for zone in [New_York, Tokyo] {
            let next = from_tick(schedule().each().day().at("10:30").build(), from, &zone).unwrap();
            assert_eq!((next.hour(), next.minute(), next.second()), (10, 30, 0));
            assert!(next > from);
        }
    }

    #[test]
    fn builder_calendar_days_preserve_time_across_dst() {
        use chrono_tz::America::New_York;
        for (month, day, hours) in [(3, 7, 23), (10, 31, 25)] {
            let from = New_York
                .with_ymd_and_hms(2026, month, day, 10, 30, 0)
                .unwrap();
            for at in [false, true] {
                let mut builder = schedule().each().day();
                if at {
                    builder = builder.at("10:30");
                }
                let next = from_tick(builder.build(), from.with_timezone(&Utc), &New_York).unwrap();
                assert_eq!((next.hour(), next.minute()), (10, 30));
                assert_eq!(next.signed_duration_since(from).num_hours(), hours);
            }
        }
    }

    #[test]
    fn builder_rejects_ambiguous_and_nonexistent_local_times() {
        use chrono_tz::America::New_York;
        for (month, day, at) in [(3, 7, "02:30"), (10, 31, "01:30")] {
            let from = New_York
                .with_ymd_and_hms(2026, month, day, 12, 0, 0)
                .unwrap();
            assert!(
                from_tick(
                    schedule().each().day().at(at).build(),
                    from.with_timezone(&Utc),
                    &New_York
                )
                .is_none()
            );
        }
    }

    #[test]
    fn builder_weekdays_select_weeks_and_keep_intervals() {
        let from = Utc.with_ymd_and_hms(2026, 1, 5, 10, 30, 0).unwrap();
        for weekday in [schedule().each().monday(), schedule().each().day().monday()] {
            let next = from_tick(weekday.at("10:30").build(), from, &Utc).unwrap();
            assert_eq!(next.signed_duration_since(from), chrono::Duration::weeks(1));
        }
        let next = from_tick(
            schedule().every(2).minutes().monday().at("10:30").build(),
            from,
            &Utc,
        )
        .unwrap();
        assert_eq!(next.signed_duration_since(from), chrono::Duration::weeks(2));
        let weekdays = [
            schedule().each().monday(),
            schedule().each().tuesday(),
            schedule().each().wednesday(),
            schedule().each().thursday(),
            schedule().each().friday(),
            schedule().each().saturday(),
            schedule().each().sunday(),
        ];
        for (day, builder) in weekdays.into_iter().enumerate() {
            let next = from_tick(builder.build(), from, &Utc).unwrap();
            assert_eq!(next.weekday().num_days_from_monday() as usize, day);
            assert!(next > from);
        }
    }

    #[test]
    fn builder_rejects_date_overflow_and_preserves_elapsed_intervals() {
        let from = DateTime::<Utc>::MAX_UTC;
        for builder in [
            schedule().each().minute(),
            schedule().each().hour(),
            schedule().each().day(),
            schedule().each().week(),
        ] {
            assert!(from_tick(builder.build(), from, &Utc).is_none());
        }
        assert!(
            from_tick(
                schedule().each().day().build(),
                from,
                &chrono_tz::Asia::Tokyo
            )
            .is_none()
        );
        use chrono_tz::America::New_York;
        let from = New_York.with_ymd_and_hms(2026, 3, 8, 1, 59, 0).unwrap();
        let next = from_tick(
            schedule().every(2).minutes().build(),
            from.with_timezone(&Utc),
            &New_York,
        )
        .unwrap();
        assert_eq!(
            next.signed_duration_since(from),
            chrono::Duration::minutes(2)
        );
        assert_eq!((next.hour(), next.minute()), (3, 1));
    }

    #[test]
    fn test_schedule_examples() {
        // Every 10 minutes
        let s1 = schedule().every(10).minutes().build::<Utc>();
        println!("{}", s1);

        // Every hour
        let s2 = schedule().each().hour().build::<Utc>();
        println!("{}", s2);

        // Every day at 10:30
        let s3 = schedule().each().day().at("10:30").build::<Utc>();
        println!("{}", s3);

        // Every Monday
        let s4 = schedule().each().monday().build::<Utc>();
        println!("{}", s4);

        // Every Wednesday at 13:15
        let s5 = schedule().each().wednesday().at("13:15").build::<Utc>();
        println!("{}", s5);

        // Every minute at second 17
        let s7 = schedule().each().minute().at(":17").build::<Utc>();
        println!("{}", s7);
    }

    #[tokio::test]
    async fn basic_worker() {
        let schedule = schedule().each().minute().at(":06").build();
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
