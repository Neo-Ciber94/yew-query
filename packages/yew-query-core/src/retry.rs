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
