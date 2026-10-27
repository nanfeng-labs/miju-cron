use futures_util::StreamExt;
use miju_core::{
    backend::{Backend, BackendConfig, TryNewBackend},
    worker::{builder::WorkerBuilder, context::WorkerContext},
};
use miju_cron::{Config, CronScheduler, Error, Schedule, Tick, timezone::Utc};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake},
    time::{Duration, SystemTime},
};

#[derive(Debug, Clone)]
struct Finite {
    ticks: VecDeque<u64>,
    calls: Arc<AtomicUsize>,
}

impl Schedule<Utc> for Finite {
    fn next_tick(&mut self, _: &Utc) -> Option<Tick> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.ticks.pop_front().map(Tick::new)
    }
}

fn finite(ticks: impl IntoIterator<Item = u64>) -> (CronScheduler<Finite>, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    (
        CronScheduler::new(Finite {
            ticks: ticks.into_iter().collect(),
            calls: calls.clone(),
        }),
        calls,
    )
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Default)]
struct Signal(AtomicUsize);

impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn extreme_timestamp_returns_error_on_first_poll() {
    let (backend, calls) = finite([u64::MAX]);
    let mut stream = std::pin::pin!(backend.into_stream());
    let mut cx = Context::from_waker(std::task::Waker::noop());
    for _ in 0..2 {
        assert!(matches!(
            futures_util::Stream::poll_next(stream.as_mut(), &mut cx),
            Poll::Ready(Some(Err(Error::InvalidTimestamp { tick: u64::MAX })))
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn overdue_ticks_are_delivered_in_order_and_source_exhausts() {
    let (backend, _) = finite([1, 2]);
    let stream = backend.into_stream();
    futures_util::pin_mut!(stream);
    for timestamp in [1, 2] {
        let tick = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(tick.get_timestamp(), timestamp);
    }
    assert!(stream.next().await.is_none());
}

#[test]
fn exhaustion_repoll_and_close_match_reference() {
    let (mut backend, calls) = finite([]);
    let mut cx = Context::from_waker(std::task::Waker::noop());
    let ctx = WorkerContext::new("empty");
    assert!(backend.poll_ready(&mut cx, &ctx).is_ready());
    assert!(matches!(
        backend.poll_next(&mut cx, &ctx),
        Poll::Ready(None)
    ));
    assert!(matches!(
        backend.poll_next(&mut cx, &ctx),
        Poll::Ready(None)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(matches!(
        backend.poll_close(&mut cx, &ctx),
        Poll::Ready(Ok(()))
    ));
    assert!(matches!(
        backend.poll_next(&mut cx, &ctx),
        Poll::Ready(None)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn timer_wakes_then_emits_late_with_scheduled_id_and_prefetches() {
    let timestamp = now() + 2;
    let (mut backend, calls) = finite([timestamp]);
    let signal = Arc::new(Signal::default());
    let wk = std::task::Waker::from(signal.clone());
    let mut cx = Context::from_waker(&wk);
    let ctx = WorkerContext::new("timer");
    assert!(backend.poll_next(&mut cx, &ctx).is_pending());
    assert!(backend.poll_next(&mut cx, &ctx).is_pending());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // Real monotonic timer: deliberately withhold polling through the deadline.
    tokio::time::timeout(Duration::from_secs(4), async {
        while signal.0.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(now() >= timestamp);
    // Closing acknowledges immediately without discarding a ready cached tick.
    assert!(matches!(
        backend.poll_close(&mut cx, &ctx),
        Poll::Ready(Ok(()))
    ));
    let Poll::Ready(Some(Ok(task))) = backend.poll_next(&mut cx, &ctx) else {
        panic!("timer did not produce task")
    };
    assert_eq!(task.args.get_timestamp(), timestamp);
    assert_eq!(
        task.ctx()
            .task_id()
            .unwrap()
            .as_ulid()
            .unwrap()
            .timestamp_ms(),
        timestamp * 1000
    );
    assert_eq!(task.ctx().run_at(), None);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(matches!(
        backend.poll_next(&mut cx, &ctx),
        Poll::Ready(None)
    ));
}

#[tokio::test(start_paused = true)]
async fn tokio_virtual_time_does_not_control_dependency_timer() {
    let (mut backend, _) = finite([now() + 60]);
    let mut cx = Context::from_waker(std::task::Waker::noop());
    let ctx = WorkerContext::new("clock-probe");
    assert!(backend.poll_next(&mut cx, &ctx).is_pending());
    tokio::time::advance(Duration::from_secs(120)).await;
    assert!(backend.poll_next(&mut cx, &ctx).is_pending());
    drop(backend);
}

#[test]
fn clone_and_timezone_reset_cached_state_but_keep_advanced_schedule() {
    let (mut backend, calls) = finite([now() + 60]);
    let ctx = WorkerContext::new("clone");
    let mut cx = Context::from_waker(std::task::Waker::noop());
    assert!(backend.poll_next(&mut cx, &ctx).is_pending());
    let mut cloned = backend.clone();
    assert!(matches!(cloned.poll_next(&mut cx, &ctx), Poll::Ready(None)));
    let mut changed = backend.with_timezone(Utc);
    assert!(matches!(
        changed.poll_next(&mut cx, &ctx),
        Poll::Ready(None)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let restored =
        CronScheduler::try_new(Config::new(changed.config().schedule().clone(), Utc)).unwrap();
    assert!(restored.config().schedule().ticks.is_empty());
}

#[tokio::test]
async fn finite_stream_and_worker_execute_and_end() {
    let timestamp = now() + 2;
    let (backend, _) = finite([timestamp]);
    let stream = backend.into_stream();
    futures_util::pin_mut!(stream);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(4), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .get_timestamp(),
        timestamp
    );
    assert!(stream.next().await.is_none());

    let timestamp = now() + 2;
    let (backend, _) = finite([timestamp]);
    let seen = Arc::new(AtomicUsize::new(0));
    let observed = seen.clone();
    let worker = WorkerBuilder::new("finite-worker")
        .backend(backend)
        .build(move |tick: Tick| {
            let observed = observed.clone();
            async move {
                assert_eq!(tick.get_timestamp(), timestamp);
                observed.fetch_add(1, Ordering::SeqCst);
            }
        });
    let completion = tokio::time::timeout(Duration::from_secs(5), worker.run()).await;
    assert_eq!(
        seen.load(Ordering::SeqCst),
        1,
        "task must execute before EOF shutdown"
    );
    completion.unwrap().unwrap();
}

#[tokio::test]
async fn worker_executes_and_honors_explicit_stop() {
    use miju_core::worker::{event::Event, ext::event_listener::EventListenerExt};
    let timestamp = now() + 2;
    let (backend, _) = finite([timestamp]);
    let seen = Arc::new(AtomicUsize::new(0));
    let observed = seen.clone();
    let worker = WorkerBuilder::new("stopped-worker")
        .backend(backend)
        .on_event(|ctx, event| {
            if matches!(event, Event::Success(_)) {
                ctx.stop().unwrap();
            }
        })
        .build(move |tick: Tick| {
            let observed = observed.clone();
            async move {
                assert_eq!(tick.get_timestamp(), timestamp);
                observed.fetch_add(1, Ordering::SeqCst);
            }
        });
    tokio::time::timeout(Duration::from_secs(4), worker.run())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(seen.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn prefetched_tick_is_delivered_after_consumer_backpressure() {
    let first = now() + 2;
    let second = first + 1;
    let (mut backend, calls) = finite([first, second]);
    let ctx = WorkerContext::new("backpressure");
    let task = tokio::time::timeout(
        Duration::from_secs(4),
        futures_util::future::poll_fn(|cx| backend.poll_next(cx, &ctx)),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    assert_eq!(task.args.get_timestamp(), first);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    // Withhold polling until the prefetched deadline has passed: no timer exists for it yet.
    tokio::time::sleep(Duration::from_secs(2)).await;
    let task = tokio::time::timeout(
        Duration::from_secs(1),
        futures_util::future::poll_fn(|cx| backend.poll_next(cx, &ctx)),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    assert_eq!(task.args.get_timestamp(), second);
    assert_eq!(
        task.ctx()
            .task_id()
            .unwrap()
            .as_ulid()
            .unwrap()
            .timestamp_ms(),
        second * 1000
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(
        futures_util::future::poll_fn(|cx| backend.poll_next(cx, &ctx))
            .await
            .is_none()
    );
}

#[cfg(feature = "serde")]
#[tokio::test]
async fn pipe_delivers_tick_to_file_backend_worker() {
    use miju_core::{
        backend::ext::BackendExt,
        worker::{event::Event, ext::event_listener::EventListenerExt},
    };
    let timestamp = now() + 2;
    let (backend, _) = finite([timestamp]);
    let destination = miju_file_storage::JsonStorage::new_temp().unwrap();
    let seen = Arc::new(AtomicUsize::new(0));
    let observed = seen.clone();
    let worker = WorkerBuilder::new("pipe-worker")
        .backend(backend.pipe_to(destination))
        .on_event(|ctx, event| {
            if matches!(event, Event::Success(_)) {
                ctx.stop().unwrap();
            }
        })
        .build(move |tick: Tick| {
            let observed = observed.clone();
            async move {
                assert_eq!(tick.get_timestamp(), timestamp);
                observed.fetch_add(1, Ordering::SeqCst);
            }
        });
    tokio::time::timeout(Duration::from_secs(5), worker.run())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(seen.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "cron")]
#[tokio::test]
async fn every_second_worker_survives_three_second_tasks_at_single_concurrency() {
    use miju_core::worker::{event::Event, ext::event_listener::EventListenerExt};
    use std::str::FromStr;
    let completed = Arc::new(AtomicUsize::new(0));
    let observed = completed.clone();
    let worker = WorkerBuilder::new("slow-cron-worker")
        .backend(CronScheduler::new(
            cron::Schedule::from_str("* * * * * *").unwrap(),
        ))
        .layer(tower::limit::ConcurrencyLimitLayer::new(1))
        .on_event(move |ctx, event| {
            if matches!(event, Event::Success(_))
                && observed.fetch_add(1, Ordering::SeqCst) + 1 == 3
            {
                ctx.stop().unwrap();
            }
        })
        .build(|_: Tick| async {
            tokio::time::sleep(Duration::from_secs(3)).await;
        });
    tokio::time::timeout(Duration::from_secs(20), worker.run())
        .await
        .expect("worker should stop after three successful tasks")
        .expect("consumer backpressure must not fail the worker");
    assert_eq!(completed.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn overdue_tick_does_not_skip_future_or_invalid_tick() {
    let future = now() + 2;
    let (backend, calls) = finite([0, future, u64::MAX]);
    let stream = backend.into_stream();
    futures_util::pin_mut!(stream);
    assert_eq!(stream.next().await.unwrap().unwrap().get_timestamp(), 0);
    let mut cx = Context::from_waker(std::task::Waker::noop());
    assert!(futures_util::Stream::poll_next(stream.as_mut(), &mut cx).is_pending());
    let tick = tokio::time::timeout(Duration::from_secs(4), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(tick.get_timestamp(), future);
    assert!(now() >= future);
    for _ in 0..2 {
        assert!(matches!(
            stream.next().await,
            Some(Err(Error::InvalidTimestamp { tick: u64::MAX }))
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn overdue_finite_worker_drains_serially_and_reaches_eof() {
    let (backend, _) = finite([0, 1, 2]);
    let completed = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let observed = completed.clone();
    let worker = WorkerBuilder::new("overdue-finite-worker")
        .backend(backend)
        .layer(tower::limit::ConcurrencyLimitLayer::new(1))
        .build(move |tick: Tick| {
            let observed = observed.clone();
            let active = active.clone();
            async move {
                assert_eq!(active.fetch_add(1, Ordering::SeqCst), 0);
                assert_eq!(tick.get_timestamp(), observed.load(Ordering::SeqCst) as u64);
                tokio::time::sleep(Duration::from_millis(20)).await;
                observed.fetch_add(1, Ordering::SeqCst);
                active.fetch_sub(1, Ordering::SeqCst);
            }
        });
    tokio::time::timeout(Duration::from_secs(3), worker.run())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.load(Ordering::SeqCst), 3);
}
