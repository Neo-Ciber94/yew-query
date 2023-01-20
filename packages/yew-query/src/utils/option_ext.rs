pub trait OptionExt<T> {
    /// Updates the inner value of an `Option<T>`.
    fn update<F>(&mut self, f: F)
    where
        F: FnOnce(T) -> T;
}

impl<T> OptionExt<T> for Option<T> {
    fn update<F>(&mut self, f: F)
    where
        F: FnOnce(T) -> T,
    {
        match self.take() {
            Some(x) => {
                let new_value = f(x);
                self.replace(new_value);
            }
            None => {}
        }
    }
}
