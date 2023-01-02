use instant::Duration;
use prokio::{
    pinned::oneshot::{channel, Sender},
    spawn_local,
};

pub struct Timeout {
    signal: Option<Sender<()>>,
}

impl Timeout {
    pub fn new<F>(duration: Duration, f: F) -> Self
    where
        F: Fn() + 'static,
    {
        let (sx, rx) = channel();

        let timeout = async move {
            prokio::time::sleep(duration).await;
            f();
        };

        spawn_local(async move {
            tokio::select! {
                _ = rx => {},
                _ = timeout => {}
            };
        });

        Timeout { signal: Some(sx) }
    }

    pub fn clear(mut self) {
        self.clear_timeout();
    }

    fn clear_timeout(&mut self) {
        if let Some(sx) = self.signal.take() {
            let _ = sx.send(()); // We ignore if the channel is close
        }
    }
}

impl Drop for Timeout {
    fn drop(&mut self) {
        self.clear_timeout();
    }
}
