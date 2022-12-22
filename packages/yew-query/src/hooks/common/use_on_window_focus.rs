use super::use_is_first_render::use_is_first_render;
use crate::listener::EventListener;
use yew::{use_effect_with_deps, hook};

#[hook]
pub fn use_on_window_focus<F>(callback: F)
where
    F: Fn() + 'static,
{
    let first_render = use_is_first_render();

    use_effect_with_deps(
        move |first_render| {
            let first_render = *first_render;
            let listener = EventListener::window("focus", move |_| {
                if first_render {
                    return;
                }

                callback();
            });

            move || {
                listener.unsubscribe();
            }
        },
        first_render,
    );
}
