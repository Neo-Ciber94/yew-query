use std::rc::Rc;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{window, Element, Event};

/// A listener to an element event.
#[derive(Clone)]
pub struct Listener {
    event: String,
    element: Element,
    closure: Option<Rc<Closure<dyn FnMut(Event)>>>,
}

impl Listener {
    /// Creates a listener to the given element event.
    pub fn new<F>(event: &str, element: Element, mut f: F) -> Self
    where
        F: FnMut(Event) + 'static,
    {
        //let window = window().unwrap();
        let event = event.to_owned();
        let closure = Rc::new(Closure::wrap(
            Box::new(move |e: Event| f(e)) as Box<dyn FnMut(_)>
        ));

        log::trace!("adding listener to `{event}` in `{element:?}`");

        {
            let closure = closure.clone();
            element
                .add_event_listener_with_callback(
                    event.as_str(),
                    &(&*closure).as_ref().unchecked_ref(),
                )
                .unwrap();
        }

        Listener {
            event,
            element,
            closure: Some(closure),
        }
    }

    /// Creates a listener to a `window` event.
    pub fn window<F>(event: &str, f: F) -> Self
    where
        F: FnMut(Event) + 'static,
    {
        let window = window().unwrap().dyn_into().unwrap();
        Self::new(event, window, f)
    }

    /// Unsubscribe from the event.
    pub fn unsubscribe(mut self) {
        self.drop_and_remove_listener();
    }

    fn drop_and_remove_listener(&mut self) {
        if let Some(closure) = &self.closure.take() {
            let element = &self.element;
            let event = self.event.as_str();
            let closure = closure.clone();

            element
                .remove_event_listener_with_callback(event, &(&*closure).as_ref().unchecked_ref())
                .unwrap();

            log::trace!("removing listening to `{event}` in `{element:?}`");
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Some(closure) = &self.closure {
            if Rc::strong_count(closure) == 1 {
                return;
            }

            log::trace!(
                "removing listening to `{}` in `{:?}`",
                self.event,
                self.element
            );
            self.drop_and_remove_listener();
        }
    }
}
