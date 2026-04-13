#![recursion_limit = "512"]

pub mod app;
pub mod i18n;
pub mod models;
pub mod pages;
pub mod server_fns;

#[cfg(feature = "ssr")]
pub use rustpos_common::printer;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
