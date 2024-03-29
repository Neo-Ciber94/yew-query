use web_sys::AbortController;
use yew::{hook, use_effect_with_deps, use_mut_ref};

use super::use_is_first_render;

#[hook]
pub fn use_abort_controller() -> AbortController {
    let controller_ref = use_mut_ref(get_abort_controller);
    let controller = { controller_ref.borrow().clone() };
    let is_first_render = use_is_first_render();

    use_effect_with_deps(
        move |_| {
            if is_first_render {
                return;
            }

            *controller_ref.borrow_mut() = get_abort_controller();
        },
        (),
    );

    controller
}

fn get_abort_controller() -> AbortController {
    AbortController::new().expect("expected `AbortController`")
}
