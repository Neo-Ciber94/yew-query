use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use instant::Duration;
use prokio::spawn_local;

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

        {
            let cancel = cancel.clone();
            spawn_local(async move {
                while !cancel.load(Ordering::SeqCst) {
                    prokio::time::sleep(duration).await;

                    if !cancel.load(Ordering::SeqCst) {
                        f();
                    } else {
                        break;
                    }
                }
            });
        }

        Interval { cancel }
    }

    pub fn cancel(mut self) {
        self.clear_interval();
    }

    fn clear_interval(&mut self) {
        if let Err(_) =
            self.cancel
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        {
            //
        }
    }
}

impl Drop for Interval {
    fn drop(&mut self) {
        self.clear_interval();
    }
}

pub fn run_interval<F>(duration: Duration, f: F) -> Interval
where
    F: Fn() + 'static,
{
    Interval::new(duration, f)
}
