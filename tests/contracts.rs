use miju_cron::{Config, Error, Schedule, Tick};
use std::time::Duration;

#[test]
fn tick_order_and_schedule_exhaustion() {
    #[derive(Debug, PartialEq, PartialOrd)]
    struct Zone;

    struct Once(Option<Tick<Zone>>);

    impl Schedule<Zone> for Once {
        fn next_tick(&mut self, _: &Zone) -> Option<Tick<Zone>> {
            self.0.take()
        }
    }

    let mut schedule = Once(Some(Tick::new(42)));
    assert_eq!(schedule.next_tick(&Zone).unwrap().get_timestamp(), 42);
    assert!(schedule.next_tick(&Zone).is_none());
    assert!(Tick::<Zone>::new(41) < Tick::<Zone>::new(42));
    let config = Config::new(String::from("rule"), 5_u8);
    let cloned = config.clone();
    assert_eq!(cloned.schedule(), "rule");
    assert_eq!(*cloned.timezone(), 5);
}

#[test]
fn error_preserves_duration_and_tick() {
    let error = Error::OutOfRange {
        duration: Duration::from_millis(1500),
        tick: 42,
    };
    assert_eq!(
        error.to_string(),
        "Tick out of range for 42 (past duration:  1.5s)"
    );
}

#[cfg(feature = "serde")]
#[test]
fn config_wire_contract_and_errors() {
    let value = Config::new(42_u32, 8_i32);
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        r#"{"schedule":"42","timezone":"8"}"#
    );
    let decoded: Config<u32, i32> =
        serde_json::from_str(r#"{"timezone":"8","schedule":"42"}"#).unwrap();
    assert_eq!(*decoded.schedule(), 42);
    assert_eq!(*decoded.timezone(), 8);
    for (json, message) in [
        (r#"{"schedule":"42"}"#, "missing field `timezone`"),
        (r#"{"timezone":"8"}"#, "missing field `schedule`"),
        (
            r#"{"schedule":"42","schedule":"43","timezone":"8"}"#,
            "duplicate field `schedule`",
        ),
        (
            r#"{"schedule":"42","timezone":"8","timezone":"9"}"#,
            "duplicate field `timezone`",
        ),
        (r#"{"schedule":"bad","timezone":"8"}"#, "invalid digit"),
        (r#"{"schedule":"42","timezone":"bad"}"#, "invalid digit"),
        (
            r#"{"schedule":"42","timezone":"8","extra":"x"}"#,
            "unknown field",
        ),
        (r#"{"schedule":42,"timezone":"8"}"#, "invalid type"),
    ] {
        let error = serde_json::from_str::<Config<u32, i32>>(json).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    let tick = Tick::<()>::new(42);
    assert_eq!(
        serde_json::to_string(&tick).unwrap(),
        r#"{"timestamp":42,"_marker":null}"#
    );
    assert_eq!(
        serde_json::from_str::<Tick<()>>(&serde_json::to_string(&tick).unwrap()).unwrap(),
        tick
    );
}

#[cfg(feature = "chrono")]
#[test]
fn timezone_preserves_dst_ambiguity_and_gap() {
    use chrono::{LocalResult, TimeZone};
    use miju_cron::timezone::America::New_York;
    assert!(matches!(
        New_York.with_ymd_and_hms(2026, 3, 8, 2, 30, 0),
        LocalResult::None
    ));
    assert!(matches!(
        New_York.with_ymd_and_hms(2026, 11, 1, 1, 30, 0),
        LocalResult::Ambiguous(_, _)
    ));
}

#[cfg(feature = "serde")]
#[test]
fn config_sequence_contract_and_errors() {
    let decoded: Config<u32, i32> = serde_json::from_str(r#"["42","8"]"#).unwrap();
    assert_eq!((*decoded.schedule(), *decoded.timezone()), (42, 8));
    for (json, message) in [
        ("[]", "invalid length 0"),
        (r#"["42"]"#, "invalid length 1"),
        (r#"["42","8","extra"]"#, "trailing characters"),
        (r#"[42,"8"]"#, "invalid type"),
        (r#"["42",8]"#, "invalid type"),
        (r#"["bad","8"]"#, "invalid digit"),
        (r#"["42","bad"]"#, "invalid digit"),
    ] {
        let error = serde_json::from_str::<Config<u32, i32>>(json).unwrap_err();
        assert!(error.to_string().contains(message), "{json}: {error}");
    }
}
