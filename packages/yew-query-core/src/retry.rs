use std::{fmt::Debug, rc::Rc, time::Duration};

type Retry = Box<dyn Iterator<Item = Duration>>;

/// Boxes a retry iterator.
#[derive(Clone)]
pub struct Retryer(Rc<dyn Fn() -> Retry>);

impl Retryer {
    /// Constructs a new `Retryer`.
    pub fn new<F, I>(f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: Iterator<Item = Duration> + 'static,
    {
        let f = Rc::new(move || {
            let retry = f();
            Box::new(retry) as Retry
        });

        Retryer(f)
    }

    /// Returns an iterator over a duration used for retrying an operation.
    pub fn get(&self) -> impl Iterator<Item = Duration> {
        (self.0)()
    }
}

impl Debug for Retryer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Retryer")
    }
}
