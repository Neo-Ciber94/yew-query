use std::{fmt::Debug, rc::Rc, time::Duration};

type DurationIterator = Box<dyn Iterator<Item = Duration>>;

/// Boxes a retry iterator.
#[derive(Clone)]
pub struct Retry(Rc<dyn Fn() -> DurationIterator>);

impl Retry {
    /// Constructs a new `Retry`.
    pub fn new<F, I>(f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: Iterator<Item = Duration> + 'static,
    {
        let f = Rc::new(move || {
            let retry = f();
            Box::new(retry) as DurationIterator
        });

        Retry(f)
    }

    /// Returns an iterator over a duration used for retrying an operation.
    pub fn get(&self) -> impl Iterator<Item = Duration> {
        (self.0)()
    }
}

impl Debug for Retry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Retry")
    }
}

impl IntoIterator for Retry {
    type Item = Duration;
    type IntoIter = Box<dyn Iterator<Item = Duration>>;

    fn into_iter(self) -> Self::IntoIter {
        (self.0)()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Retry;

    #[test]
    fn retry_sleep_test() {
        let retry = Retry::new(move || std::iter::repeat(Duration::from_millis(100)).take(3));
        let start = Instant::now();
        
        for t in retry {
            std::thread::sleep(t);
        }

        let dur = Instant::now() - start;
        assert!(dur >= Duration::from_millis(300), "duration: {:?}", dur);
    }
}
