use std::rc::Rc;

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{window, Event};
use yew::use_effect_with_deps;

use super::use_is_first_render::use_is_first_render;

pub fn use_on_window_focus<F>(callback: F)
where
    F: Fn() + 'static,
{
    let first_render = use_is_first_render();

    use_effect_with_deps(
        move |first_render| {
            let window = window().unwrap();
            let closure = Rc::new(Closure::wrap(Box::new(move |_: Event| {
                callback();
            }) as Box<dyn FnMut(_)>));

            let cleanup = {
                let window = window.clone();
                let closure = closure.clone();
                move || {
                    window
                        .remove_event_listener_with_callback(
                            "focus",
                            &(&*closure).as_ref().unchecked_ref(),
                        )
                        .unwrap();
                }
            };

            if *first_render {
                return cleanup;
            }

            window
                .add_event_listener_with_callback("focus", &(&*closure).as_ref().unchecked_ref())
                .unwrap();

            cleanup
        },
        first_render,
    );
}
