use chrono::{Duration, Local, NaiveTime, TimeZone};
use miju::prelude::*;
use miju_cron::{CronScheduler, Schedule, Tick};

/// Daily routine at 8am
#[derive(Debug, Clone)]
struct MyDailyRoutine;

impl Schedule<Local> for MyDailyRoutine {
    fn next_after(&self, after: &Tick<Local>, tz: &Local) -> Option<Tick<Local>> {
        let after = tz
            .timestamp_opt(i64::try_from(after.get_timestamp()).ok()?, 0)
            .single()?;
        let eight_am = NaiveTime::from_hms_opt(8, 0, 0)?;

        // Today's 8:00 AM if it is still ahead, otherwise tomorrow's.
        [after.date_naive(), after.date_naive() + Duration::days(1)]
            .into_iter()
            .filter_map(|day| day.and_time(eight_am).and_local_timezone(*tz).single())
            .find(|candidate| *candidate > after)
            .and_then(|candidate| u64::try_from(candidate.timestamp()).ok())
            .map(Tick::new)
    }
}

async fn handle_tick(tick: Tick<Local>, data: Data<usize>) -> Result<(), BoxDynError> {
    println!("Handling tick: {:?} with data: {:?}", tick, data);
    Ok(())
}

#[tokio::main]
async fn main() {
    let backend = CronScheduler::new(MyDailyRoutine).with_timezone(Local);
    let worker = WorkerBuilder::new("morning-cereal")
        .backend(backend)
        .data(42usize)
        .build(handle_tick);

    worker.run().await.unwrap();
}
