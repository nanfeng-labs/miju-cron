#[cfg(feature = "builder")]
#[test]
fn builder_preserves_interval_and_timezone_behavior() {
    use miju_cron::{
        Schedule,
        builder::schedule,
        timezone::{America::New_York, Asia::Tokyo},
    };
    let mut every_two = schedule().every(2).minutes().build::<chrono::Utc>();
    let first = every_two.next_tick(&chrono::Utc).unwrap().get_timestamp();
    let second = every_two.next_tick(&chrono::Utc).unwrap().get_timestamp();
    // The reference multiplies the stored interval by Minutes(interval).
    assert_eq!(second - first, 4 * 60);
    let mut at = schedule()
        .every(2)
        .minutes()
        .at(":17")
        .build::<chrono::Utc>();
    let first = at.next_tick(&chrono::Utc).unwrap().get_timestamp();
    let second = at.next_tick(&chrono::Utc).unwrap().get_timestamp();
    assert_eq!(second - first, 60);
    assert_eq!(first % 60, 17);
    let mut daily = schedule().each().day().at("10:30").build::<chrono_tz::Tz>();
    daily.next_tick(&New_York).unwrap();
    let mut same_start = daily.clone();
    assert_eq!(
        daily.next_tick(&New_York).unwrap().get_timestamp(),
        same_start.next_tick(&Tokyo).unwrap().get_timestamp()
    );
    let mut weekday_without_unit = schedule().each().monday().build::<chrono::Utc>();
    assert!(weekday_without_unit.next_tick(&chrono::Utc).is_none());
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
fn builder_zero_interval_retains_timestamp() {
    use miju_cron::{Schedule, builder::schedule};
    let mut zero = schedule().every(0).minutes().build::<chrono::Utc>();
    let first = zero.next_tick(&chrono::Utc).unwrap().get_timestamp();
    let second = zero.next_tick(&chrono::Utc).unwrap().get_timestamp();
    assert_eq!(first, second);
}

#[cfg(all(feature = "builder", debug_assertions))]
#[test]
#[should_panic(expected = "attempt to multiply with overflow")]
fn builder_large_interval_preserves_reference_debug_overflow() {
    use miju_cron::{Schedule, builder::schedule};
    schedule()
        .every(65_536)
        .minutes()
        .build::<chrono::Utc>()
        .next_tick(&chrono::Utc);
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
