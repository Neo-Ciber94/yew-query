use std::rc::Rc;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{window, Event, EventTarget};

/// A listener to an element event.
#[derive(Clone)]
pub struct EventListener {
    event: String,
    target: EventTarget,
    closure: Option<Rc<Closure<dyn FnMut(Event)>>>,
}

impl EventListener {
    /// Creates a listener to the given element target.
    pub fn new<F>(event: &str, target: EventTarget, f: F) -> Self
    where
        F: Fn(Event) + 'static,
    {
        //let window = window().unwrap();
        let event = event.to_owned();
        let closure = Rc::new(Closure::wrap(
            Box::new(move |e: Event| f(e)) as Box<dyn FnMut(_)>
        ));

        {
            let closure = closure.clone();
            target
                .add_event_listener_with_callback(
                    event.as_str(),
                    &(&*closure).as_ref().unchecked_ref(),
                )
                .unwrap();
        }

        EventListener {
            event,
            target,
            closure: Some(closure),
        }
    }

    /// Creates a listener to a `window` event.
    pub fn window<F>(event: &str, f: F) -> Self
    where
        F: Fn(Event) + 'static,
    {
        let window = window().unwrap().dyn_into().expect("failed to cast window");
        Self::new(event, window, f)
    }

    /// Returns the event being listened.
    pub fn event(&self) -> &str {
        &self.event.as_str()
    }

    /// Returns the event target.
    pub fn target(&self) -> &EventTarget {
        &self.target
    }

    /// Unsubscribe from the event.
    pub fn unsubscribe(mut self) {
        self.drop_and_remove_listener();
    }

    fn drop_and_remove_listener(&mut self) {
        if let Some(closure) = &self.closure.take() {
            let element = &self.target;
            let event = self.event.as_str();
            let closure = closure.clone();

            element
                .remove_event_listener_with_callback(event, &(&*closure).as_ref().unchecked_ref())
                .unwrap();
        }
    }
}

impl Drop for EventListener {
    fn drop(&mut self) {
        if let Some(closure) = &self.closure {
            if Rc::strong_count(closure) == 1 {
                return;
            }

            self.drop_and_remove_listener();
        }
    }
}
