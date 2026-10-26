// Run this example with `cargo run --example english --features=english`

use chrono::Local;
use miju::{layers::retry::RetryPolicy, prelude::*};
use miju_cron::english::EnglishRoutine;
use miju_cron::{CronScheduler, Tick};
use std::str::FromStr;

async fn handle_tick(tick: Tick<Local>, data: Data<usize>) -> Result<(), BoxDynError> {
    println!("Handling tick: {:?} with data: {:?}", tick, data);
    Ok(())
}

#[tokio::main]
async fn main() {
    let schedule = EnglishRoutine::from_str("in 5 seconds").unwrap();
    let backend = CronScheduler::new(schedule).with_timezone(Local);
    let worker = WorkerBuilder::new("morning-cereal")
        .backend(backend)
        .retry(RetryPolicy::retries(5))
        .data(42usize)
        .build(handle_tick);

    worker.run().await.unwrap();
}
