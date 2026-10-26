#[cfg(feature = "builder")]
#[test]
fn builder_respects_interval_and_timezone() {
    use miju_cron::{
        Schedule,
        builder::schedule,
        timezone::{America::New_York, Asia::Tokyo},
    };
    let mut every_two = schedule().every(2).minutes().build::<chrono::Utc>();
    let first = every_two.next_tick(&chrono::Utc).unwrap().get_timestamp();
    let second = every_two.next_tick(&chrono::Utc).unwrap().get_timestamp();
    assert_eq!(second - first, 2 * 60);
    let mut at = schedule()
        .every(2)
        .minutes()
        .at(":17")
        .build::<chrono::Utc>();
    let first = at.next_tick(&chrono::Utc).unwrap().get_timestamp();
    let second = at.next_tick(&chrono::Utc).unwrap().get_timestamp();
    assert_eq!(second - first, 2 * 60);
    assert_eq!(first % 60, 17);
    let mut daily = schedule().each().day().at("10:30").build::<chrono_tz::Tz>();
    daily.next_tick(&New_York).unwrap();
    let mut same_start = daily.clone();
    assert_ne!(
        daily.next_tick(&New_York).unwrap().get_timestamp(),
        same_start.next_tick(&Tokyo).unwrap().get_timestamp()
    );
    let mut weekday_without_unit = schedule().each().monday().build::<chrono::Utc>();
    assert!(weekday_without_unit.next_tick(&chrono::Utc).is_some());
    let mut weekly = schedule()
        .each()
        .week()
        .monday()
        .at("10:30")
        .build::<chrono::Utc>();
    let first = weekly.next_tick(&chrono::Utc).unwrap().get_timestamp();
    let second = weekly.next_tick(&chrono::Utc).unwrap().get_timestamp();
    assert_eq!(second - first, 7 * 86400);
}

#[cfg(feature = "english")]
#[test]
fn english_preserves_input_errors_and_tick_contract() {
    use miju_cron::{
        Schedule,
        english::{EnglishRoutine, EnglishRoutineError},
    };
    let mut routine = EnglishRoutine::new("every second").unwrap();
    assert_eq!(routine.input(), "every second");
    let before = chrono::Utc::now().timestamp();
    let tick = routine.next_tick(&chrono::Utc).unwrap();
    let after = chrono::Utc::now().timestamp();
    assert!(tick.get_timestamp() > before as u64 && tick.get_timestamp() <= after as u64 + 1);
    assert!(matches!(
        EnglishRoutine::new("this is not a schedule"),
        Err(EnglishRoutineError::InvalidStatement(_))
    ));
    assert!("every second".parse::<EnglishRoutine>().is_ok());
    assert!(EnglishRoutine::try_from(String::from("every second")).is_ok());
}

#[cfg(feature = "jiff")]
#[test]
fn jiff_schedule_reexport_and_seconds() {
    use jiff_cron::jiff::{Timestamp, tz::TimeZone};
    use miju_cron::{Schedule, jiff::JiffSchedule};
    let mut schedule: JiffSchedule = "* * * * * *".parse().unwrap();
    let before = Timestamp::now().as_second();
    let tick = schedule.next_tick(&TimeZone::UTC).unwrap();
    let after = Timestamp::now().as_second();
    assert!(tick.get_timestamp() > before as u64 && tick.get_timestamp() <= after as u64 + 1);
}

#[cfg(feature = "cron")]
#[test]
fn cron_finite_schedule_has_no_past_tick() {
    use miju_cron::Schedule;
    let mut past: cron::Schedule = "0 0 0 1 1 * 2000".parse().unwrap();
    assert!(past.next_tick(&chrono::Utc).is_none());
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
        let mut zero = builder.build::<chrono::Utc>();
        assert!(zero.next_tick(&chrono::Utc).is_none());
    }
    for at in ["bad", ":60", "25:00", "10:60", "10:30:60"] {
        assert!(
            schedule()
                .each()
                .day()
                .at(at)
                .build::<chrono::Utc>()
                .next_tick(&chrono::Utc)
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
            let mut schedule = builder.build::<chrono::Utc>();
            let first = schedule.next_tick(&chrono::Utc).unwrap().get_timestamp();
            let second = schedule.next_tick(&chrono::Utc).unwrap().get_timestamp();
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
    let mut gap: cron::Schedule = format!("0 30 2 {second_sunday} 3 * {year}")
        .parse()
        .unwrap();
    assert!(gap.next_tick(&New_York).is_none());
    let november_first = chrono::NaiveDate::from_ymd_opt(year, 11, 1).unwrap();
    let first_sunday = 1 + (7 - november_first.weekday().num_days_from_sunday()) % 7;
    let mut fold: cron::Schedule = format!("0 30 1 {first_sunday} 11 * {year}")
        .parse()
        .unwrap();
    let tick = fold.next_tick(&New_York).unwrap();
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
