//! Blocking preview and export work shares one slot without occupying the async executor.

pub(crate) async fn run<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    // ponytail: one global heavy job; split preview/export slots only if measured latency warrants it.
    static SLOT: smol::lock::Semaphore = smol::lock::Semaphore::new(1);
    let permit = SLOT.acquire().await;
    smol::unblock(move || {
        // Canceling the awaiting future must not release a still-running job's slot.
        let _permit = permit;
        work()
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    };
    use std::time::Duration;

    static TEST_LOCK: Mutex<()> = Mutex::new(());
    const DEADLINE: Duration = Duration::from_secs(10);

    #[test]
    fn blocked_work_leaves_the_async_executor_free_for_timers() {
        let _test = TEST_LOCK.lock().unwrap();
        let executor = smol::Executor::new();
        let (started_tx, started_rx) = smol::channel::bounded(1);
        let (release_tx, release_rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        let observer = std::thread::spawn(move || {
            let progress = progress_rx.recv_timeout(DEADLINE);
            let _ = release_tx.send(());
            progress
        });

        let result = smol::block_on(executor.run(async {
            let job = executor.spawn(run(move || {
                started_tx.try_send(()).unwrap();
                release_rx.recv_timeout(DEADLINE + DEADLINE).unwrap();
                42
            }));
            started_rx.recv().await.unwrap();
            smol::Timer::after(Duration::from_millis(1)).await;
            let _ = progress_tx.send(());
            job.await
        }));

        assert_eq!(result, 42);
        assert!(
            observer.join().unwrap().is_ok(),
            "blocking work starved the timer"
        );
    }

    #[test]
    fn canceling_a_waiter_keeps_its_running_work_inside_the_global_cap() {
        let _test = TEST_LOCK.lock().unwrap();
        let active = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let first_active = active.clone();
        let mut first = Box::pin(run(move || {
            first_active.fetch_add(1, Ordering::SeqCst);
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(DEADLINE).unwrap();
            first_active.fetch_sub(1, Ordering::SeqCst);
            finished_tx.send(()).unwrap();
        }));
        assert!(smol::block_on(smol::future::poll_once(first.as_mut())).is_none());
        started_rx.recv_timeout(DEADLINE).unwrap();
        drop(first);

        let (next_started_tx, next_started_rx) = mpsc::channel();
        let mut next = Box::pin(run(move || {
            let already_active = active.fetch_add(1, Ordering::SeqCst);
            next_started_tx.send(already_active).unwrap();
            active.fetch_sub(1, Ordering::SeqCst);
            73
        }));
        assert!(smol::block_on(smol::future::poll_once(next.as_mut())).is_none());
        let premature_start = next_started_rx.recv_timeout(Duration::from_secs(1));
        release_tx.send(()).unwrap();
        finished_rx.recv_timeout(DEADLINE).unwrap();
        assert_eq!(smol::block_on(next), 73);
        assert!(
            matches!(premature_start, Err(mpsc::RecvTimeoutError::Timeout)),
            "another heavy job started before the canceled job finished: {premature_start:?}"
        );
        assert_eq!(next_started_rx.recv_timeout(DEADLINE).unwrap(), 0);
    }
}
