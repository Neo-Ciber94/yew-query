use std::rc::Rc;
use instant::Duration;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::window;

pub struct Timeout(i32);

impl Timeout {
    pub fn new<F>(duration: Duration, f: F) -> Self
    where
        F: Fn() + 'static,
    {
        let window = window().expect("expected `window`");
        let handler = Rc::new(Closure::wrap(Box::new(move || f()) as Box<dyn FnMut()>));
        
        // TODO: We should keep this value between 0 and i32::MAX
        let millis = i32::try_from(duration.as_millis()).unwrap_or(i32::MAX);

        let id = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &(&*handler).as_ref().unchecked_ref(),
                millis,
            )
            .expect("failed to set timeout");

        Timeout(id)
    }

    pub fn clear(mut self) {
        self.clear_timeout();
    }

    fn clear_timeout(&mut self) {
        let window = window().expect("expected `window`");
        window.clear_timeout_with_handle(self.0);
    }
}

impl Drop for Timeout {
    fn drop(&mut self) {
        self.clear_timeout();
    }
}
