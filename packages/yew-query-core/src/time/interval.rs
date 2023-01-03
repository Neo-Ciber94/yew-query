use futures::{
    stream::{AbortHandle, Abortable},
    StreamExt,
};
use instant::Duration;
use prokio::spawn_local;

#[derive(Debug, Clone)]
pub struct Interval {
    cancel: AbortHandle,
}

impl Interval {
    pub fn new<F>(duration: Duration, f: F) -> Self
    where
        F: Fn() + 'static,
    {
        let (cancel, registration) = AbortHandle::new_pair();
        let stream = prokio::time::interval(duration);
        let abortable_stream = Abortable::new(stream, registration).boxed_local();

        spawn_local(async move {
            tokio::pin!(abortable_stream);

            while let Some(_) = abortable_stream.next().await {
                f();
            }
        });

        Interval { cancel }
    }

    pub fn cancel(mut self) {
        self.clear_interval();
    }

    fn clear_interval(&mut self) {
        self.cancel.abort();
    }
}

impl Drop for Interval {
    fn drop(&mut self) {
        self.clear_interval();
    }
}
