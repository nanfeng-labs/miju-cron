use crate::{Tick, schedule::Schedule};

impl<Tz: chrono::TimeZone> Schedule<Tz> for cron::Schedule {
    fn next_tick(&mut self, timezone: &Tz) -> Option<Tick<Tz>> {
        self.upcoming(timezone.clone())
            .next()
            .map(|d| d.timestamp() as _)
            .map(Tick::new)
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::Duration};

    use cron::Schedule;
    use miju_core::{
        error::BoxDynError,
        task::task_id::TaskId,
        worker::{builder::WorkerBuilder, event::Event, ext::event_listener::EventListenerExt},
    };

    use crate::{backend::CronScheduler, tick::Tick};

    #[tokio::test]
    async fn basic_worker() {
        let schedule = Schedule::from_str("1/1 * * * * *").unwrap();
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
