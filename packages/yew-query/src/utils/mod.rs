mod option_ext;
pub use option_ext::*;

pub mod id {
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// An unique id.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Id(usize);
    impl Id {
        /// Returns an unique id.
        pub fn next() -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst) + 1;
            Id(id)
        }
    }
}
