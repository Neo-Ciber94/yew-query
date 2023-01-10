pub fn get_inner_html(id: impl AsRef<str>) -> String {
    gloo_utils::document()
        .get_element_by_id(id.as_ref())
        .unwrap_or_else(|| panic!("html element with id `{}` was not found", id.as_ref()))
        .inner_html()
}
