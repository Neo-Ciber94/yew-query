pub use atomic::*;

// FIXME: implementation is not cancelling the futures being executed
// `client::tests::query_with_refetch_test` fails with this implementation
#[allow(dead_code)]
mod abortable {
    use futures::{
        stream::{AbortHandle, Abortable},
        StreamExt,
    };
    use instant::Duration;
    use prokio::spawn_local;

    #[derive(Debug, Clone)]
    pub struct Interval {
        signal: AbortHandle,
    }

    impl Interval {
        pub fn new<F>(duration: Duration, f: F) -> Self
        where
            F: Fn() + 'static,
        {
            let (signal, registration) = AbortHandle::new_pair();
            let task = prokio::time::interval(duration);
            let abortable = Abortable::new(task, registration);

            spawn_local(async move {
                tokio::pin!(abortable);

                while let Some(_) = abortable.next().await {
                    println!("aborted: {}", abortable.is_aborted());
                    if !abortable.is_aborted() {
                        f();
                    }
                }
            });

            Interval { signal }
        }

        pub fn cancel(mut self) {
            self.clear_interval();
        }

        fn clear_interval(&mut self) {
            self.signal.abort();
        }
    }

    impl Drop for Interval {
        fn drop(&mut self) {
            self.clear_interval();
        }
    }
}

#[allow(dead_code)]
mod atomic {
    use instant::Duration;
    use prokio::spawn_local;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[derive(Debug, Clone)]
    pub struct Interval {
        cancel: Arc<AtomicBool>,
    }

    impl Interval {
        pub fn new<F>(duration: Duration, f: F) -> Self
        where
            F: Fn() + 'static,
        {
            let cancel = Arc::new(AtomicBool::new(false));

            spawn_local({
                let cancel = cancel.clone();

                async move {
                    while !cancel.load(Ordering::SeqCst) {
                        prokio::time::sleep(duration).await;

                        if !cancel.load(Ordering::SeqCst) {
                            f();
                        }
                    }
                }
            });

            Interval { cancel }
        }

        pub fn cancel(mut self) {
            self.clear_interval();
        }

        fn clear_interval(&mut self) {
            self.cancel
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .ok();

            log::trace!("abort");
        }
    }

    impl Drop for Interval {
        fn drop(&mut self) {
            self.clear_interval();
        }
    }
}
