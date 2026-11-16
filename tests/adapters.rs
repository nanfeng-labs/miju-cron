use miju_cron::Tick;

/// The current time as a cursor, matching how `CronScheduler` starts.
#[allow(dead_code)]
fn now<Tz>() -> Tick<Tz> {
    Tick::new(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
}

#[cfg(feature = "builder")]
#[test]
fn builder_respects_interval_and_timezone() {
    use miju_cron::{
        Schedule,
        builder::schedule,
        timezone::{America::New_York, Asia::Tokyo},
    };
    let every_two = schedule().every(2).minutes().build::<chrono::Utc>();
    let first = every_two
        .next_after(&now(), &chrono::Utc)
        .unwrap()
        .get_timestamp();
    let second = every_two
        .next_after(&Tick::new(first), &chrono::Utc)
        .unwrap()
        .get_timestamp();
    assert_eq!(second - first, 2 * 60);
    let at = schedule()
        .every(2)
        .minutes()
        .at(":17")
        .build::<chrono::Utc>();
    let first = at.next_after(&now(), &chrono::Utc).unwrap().get_timestamp();
    let second = at
        .next_after(&Tick::new(first), &chrono::Utc)
        .unwrap()
        .get_timestamp();
    assert_eq!(second - first, 2 * 60);
    assert_eq!(first % 60, 17);
    let daily = schedule().each().day().at("10:30").build::<chrono_tz::Tz>();
    let start = now();
    assert_ne!(
        daily.next_after(&start, &New_York).unwrap().get_timestamp(),
        daily.next_after(&start, &Tokyo).unwrap().get_timestamp()
    );
    let weekday_without_unit = schedule().each().monday().build::<chrono::Utc>();
    assert!(
        weekday_without_unit
            .next_after(&now(), &chrono::Utc)
            .is_some()
    );
    let weekly = schedule()
        .each()
        .week()
        .monday()
        .at("10:30")
        .build::<chrono::Utc>();
    let first = weekly
        .next_after(&now(), &chrono::Utc)
        .unwrap()
        .get_timestamp();
    let second = weekly
        .next_after(&Tick::new(first), &chrono::Utc)
        .unwrap()
        .get_timestamp();
    assert_eq!(second - first, 7 * 86400);
}

#[cfg(feature = "english")]
#[test]
fn english_preserves_input_errors_and_tick_contract() {
    use miju_cron::{
        Schedule,
        english::{EnglishRoutine, EnglishRoutineError},
    };
    let routine = EnglishRoutine::new("every second").unwrap();
    assert_eq!(routine.input(), "every second");
    let before = chrono::Utc::now().timestamp();
    let tick = routine.next_after(&now(), &chrono::Utc).unwrap();
    let after = chrono::Utc::now().timestamp();
    assert!(tick.get_timestamp() > before as u64 && tick.get_timestamp() <= after as u64 + 1);
    assert!(matches!(
        EnglishRoutine::new("this is not a schedule"),
        Err(EnglishRoutineError::InvalidStatement(_))
    ));
    assert!("every second".parse::<EnglishRoutine>().is_ok());
    assert!(EnglishRoutine::try_from(String::from("every second")).is_ok());
}

#[cfg(feature = "english")]
#[test]
fn english_is_case_insensitive_and_preserves_original_input() {
    use chrono::{TimeZone, Utc};
    use miju_cron::english::EnglishRoutine;
    let anchor = Utc.with_ymd_and_hms(2027, 1, 5, 10, 0, 1).unwrap();
    for (input, expected) in [
        ("EVERY SECOND", anchor + chrono::Duration::seconds(1)),
        (
            "Every Hour",
            Utc.with_ymd_and_hms(2027, 1, 5, 11, 0, 0).unwrap(),
        ),
        (
            "every day at NOON",
            Utc.with_ymd_and_hms(2027, 1, 5, 12, 0, 0).unwrap(),
        ),
        (
            "every day at Midnight",
            Utc.with_ymd_and_hms(2027, 1, 6, 0, 0, 0).unwrap(),
        ),
    ] {
        for routine in [
            EnglishRoutine::new(input).unwrap(),
            input.parse::<EnglishRoutine>().unwrap(),
            EnglishRoutine::try_from(input.to_string()).unwrap(),
        ] {
            assert_eq!(routine.input(), input);
            assert_eq!(
                routine.schedule().after(&anchor).next(),
                Some(expected),
                "{input}"
            );
        }
    }
}

#[cfg(feature = "english")]
#[test]
fn english_rejects_clock_times_with_different_minutes() {
    use miju_cron::english::{EnglishRoutine, EnglishRoutineError};
    for input in [
        "at 10:00 and 10:30",
        "at 9:30 and 11:45",
        "AT 9:30PM AND 11:45PM",
        "at 9:30am,11:45am",
        "at 9:30 am and 11pm",
        "at 9pm and 11:45",
        "between 9:30 and 11:45",
        "at noon and 9:30",
        "at midnight and 0:30",
        "at9:30and11:45",
        "🦀 提醒：at 9:30 and 11:45",
    ] {
        let error = EnglishRoutine::new(input).unwrap_err();
        assert!(
            matches!(error, EnglishRoutineError::InvalidStatement(_)),
            "{input}: {error:?}"
        );
        assert!(
            error.to_string().contains("different minutes"),
            "{input}: {error}"
        );
    }
}

#[cfg(feature = "english")]
#[test]
fn english_keeps_supported_clock_time_combinations() {
    use chrono::{TimeZone, Utc};
    use miju_cron::english::EnglishRoutine;
    let anchor = Utc.with_ymd_and_hms(2027, 1, 5, 9, 30, 1).unwrap();
    for (input, expected) in [
        (
            "at 9:30 and 11:30",
            Utc.with_ymd_and_hms(2027, 1, 5, 11, 30, 0).unwrap(),
        ),
        (
            "🦀 at 9:030 and 11:30",
            Utc.with_ymd_and_hms(2027, 1, 5, 11, 30, 0).unwrap(),
        ),
        (
            "at 9am and 11am",
            Utc.with_ymd_and_hms(2027, 1, 5, 11, 0, 0).unwrap(),
        ),
        (
            "at 9:00 and 11am",
            Utc.with_ymd_and_hms(2027, 1, 5, 11, 0, 0).unwrap(),
        ),
        (
            "at noon and midnight",
            Utc.with_ymd_and_hms(2027, 1, 5, 12, 0, 0).unwrap(),
        ),
        (
            "at 9:30 on Monday and Wednesday",
            Utc.with_ymd_and_hms(2027, 1, 6, 9, 30, 0).unwrap(),
        ),
        (
            "Run at midnight on the 1st and 15th of the month",
            Utc.with_ymd_and_hms(2027, 1, 15, 0, 0, 0).unwrap(),
        ),
    ] {
        let routine = EnglishRoutine::new(input).unwrap();
        assert_eq!(
            routine.schedule().after(&anchor).next(),
            Some(expected),
            "{input}"
        );
    }
}

#[cfg(feature = "jiff")]
#[test]
fn jiff_schedule_reexport_and_seconds() {
    use jiff_cron::jiff::{Timestamp, tz::TimeZone};
    use miju_cron::{Schedule, jiff::JiffSchedule};
    let schedule: JiffSchedule = "* * * * * *".parse().unwrap();
    let before = Timestamp::now().as_second();
    let tick = schedule.next_after(&now(), &TimeZone::UTC).unwrap();
    let after = Timestamp::now().as_second();
    assert!(tick.get_timestamp() > before as u64 && tick.get_timestamp() <= after as u64 + 1);
}

#[cfg(feature = "cron")]
#[test]
fn cron_finite_schedule_has_no_past_tick() {
    use miju_cron::Schedule;
    let past: cron::Schedule = "0 0 0 1 1 * 2000".parse().unwrap();
    assert!(past.next_after(&now(), &chrono::Utc).is_none());
}

#[cfg(feature = "builder")]
#[test]
fn builder_rejects_zero_and_invalid_times() {
    use miju_cron::{Schedule, builder::schedule};
    for at in [None, Some(":17")] {
        let mut builder = schedule().every(0).minutes();
        if let Some(at) = at {
            builder = builder.at(at);
        }
        let zero = builder.build::<chrono::Utc>();
        assert!(zero.next_after(&now(), &chrono::Utc).is_none());
    }
    for at in ["bad", ":60", "25:00", "10:60", "10:30:60"] {
        assert!(
            schedule()
                .each()
                .day()
                .at(at)
                .build::<chrono::Utc>()
                .next_after(&now(), &chrono::Utc)
                .is_none()
        );
    }
}

#[cfg(feature = "builder")]
#[test]
fn builder_large_intervals_do_not_overflow() {
    use miju_cron::{Schedule, builder::schedule};
    for n in [65_536, u32::MAX] {
        for at in [None, Some(":17")] {
            let mut builder = schedule().every(n).minutes();
            if let Some(at) = at {
                builder = builder.at(at);
            }
            let schedule = builder.build::<chrono::Utc>();
            let first = schedule
                .next_after(&now(), &chrono::Utc)
                .unwrap()
                .get_timestamp();
            let second = schedule
                .next_after(&Tick::new(first), &chrono::Utc)
                .unwrap()
                .get_timestamp();
            assert_eq!(second - first, u64::from(n) * 60);
        }
    }
}

#[cfg(feature = "cron")]
#[test]
fn cron_adapter_skips_dst_gap_and_returns_a_valid_fold_instant() {
    use chrono::{Datelike, TimeZone, Timelike};
    use miju_cron::{Schedule, timezone::America::New_York};
    let year = chrono::Utc::now().year() + 1;
    let march_first = chrono::NaiveDate::from_ymd_opt(year, 3, 1).unwrap();
    let second_sunday = 1 + (7 - march_first.weekday().num_days_from_sunday()) % 7 + 7;
    let gap: cron::Schedule = format!("0 30 2 {second_sunday} 3 * {year}")
        .parse()
        .unwrap();
    assert!(gap.next_after(&now(), &New_York).is_none());
    let november_first = chrono::NaiveDate::from_ymd_opt(year, 11, 1).unwrap();
    let first_sunday = 1 + (7 - november_first.weekday().num_days_from_sunday()) % 7;
    let fold: cron::Schedule = format!("0 30 1 {first_sunday} 11 * {year}")
        .parse()
        .unwrap();
    let tick = fold.next_after(&now(), &New_York).unwrap();
    let local = New_York
        .timestamp_opt(tick.get_timestamp() as i64, 0)
        .unwrap();
    assert_eq!(
        (
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute()
        ),
        (year, 11, first_sunday, 1, 30)
    );
}

#[cfg(all(feature = "cron", feature = "jiff", feature = "builder"))]
#[test]
fn next_after_is_strict_and_deterministic() {
    use jiff_cron::jiff::tz::TimeZone;
    use miju_cron::{Schedule, builder::schedule, jiff::JiffSchedule};
    let at = Tick::new(1_800_000_000);
    let cron: cron::Schedule = "* * * * * *".parse().unwrap();
    assert_eq!(
        cron.next_after(&at, &chrono::Utc).unwrap().get_timestamp(),
        1_800_000_001
    );
    assert_eq!(
        cron.next_after(&at, &chrono::Utc),
        cron.next_after(&at, &chrono::Utc)
    );
    let jiff: JiffSchedule = "* * * * * *".parse().unwrap();
    assert_eq!(
        jiff.next_after(&Tick::new(1_800_000_000), &TimeZone::UTC)
            .unwrap()
            .get_timestamp(),
        1_800_000_001
    );
    let minutely = schedule().each().minute().build::<chrono::Utc>();
    assert_eq!(
        minutely
            .next_after(&at, &chrono::Utc)
            .unwrap()
            .get_timestamp(),
        1_800_000_060
    );
}
