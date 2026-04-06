pub mod app;
pub mod models;
pub mod pages;
pub mod server_fns;

#[cfg(feature = "ssr")]
pub mod printer;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
