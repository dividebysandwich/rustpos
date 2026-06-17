use leptos::prelude::*;

use crate::i18n::I18n;

// When the keyboard is shown it may render below the visible area on tall forms
// (e.g. the item editor). Scroll it into view so it is always reachable.
#[cfg(target_arch = "wasm32")]
fn scroll_osk_into_view() {
    use wasm_bindgen::prelude::*;
    let cb = Closure::wrap(Box::new(move || {
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.query_selector(".osk").ok())
            .flatten()
        {
            // align to bottom of viewport so the field being edited stays visible
            el.scroll_into_view_with_bool(false);
        }
    }) as Box<dyn Fn()>);
    let _ = web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
    cb.forget();
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_osk_into_view() {}

/// Scrolls the window to the very top. Used when opening an edit form that is
/// rendered above a long list so the form is brought into view.
#[cfg(target_arch = "wasm32")]
pub fn scroll_page_to_top() {
    if let Some(w) = web_sys::window() {
        w.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn scroll_page_to_top() {}

#[component]
pub fn OnScreenKeyboard(
    on_key: impl Fn(String) + Copy + Send + 'static,
    shift: ReadSignal<bool>,
    i18n: RwSignal<I18n>,
) -> impl IntoView {
    // Runs once when the keyboard mounts (i.e. when it becomes visible)
    Effect::new(move |_| { scroll_osk_into_view(); });

    let rows_lower = vec![
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
        vec!["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
        vec!["a", "s", "d", "f", "g", "h", "j", "k", "l"],
        vec!["z", "x", "c", "v", "b", "n", "m", "."],
    ];
    let rows_upper = vec![
        vec!["!", "@", "#", "$", "%", "^", "&", "*", "(", ")"],
        vec!["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
        vec!["A", "S", "D", "F", "G", "H", "J", "K", "L"],
        vec!["Z", "X", "C", "V", "B", "N", "M", "."],
    ];

    view! {
        <div class="osk">
            {move || {
                let rows = if shift.get() { &rows_upper } else { &rows_lower };
                rows.iter().enumerate().map(|(row_idx, row)| {
                    // The home row (a s d f …) gets a half-key indent for a real-keyboard stagger
                    let row_class = if row_idx == 2 { "osk-row osk-row-home" } else { "osk-row" };
                    view! {
                        <div class=row_class>
                            {if row_idx == 3 {
                                let on_key_shift = on_key;
                                Some(view! {
                                    <button
                                        class=move || if shift.get() { "osk-key osk-key-wide osk-key-active" } else { "osk-key osk-key-wide" }
                                        on:click=move |_| on_key_shift("Shift".into())
                                    >{move || i18n.get().t("keyboard.shift")}</button>
                                })
                            } else {
                                None
                            }}
                            {row.iter().map(|key| {
                                let key_str = key.to_string();
                                let key_for_click = key_str.clone();
                                let on_key_inner = on_key;
                                view! {
                                    <button class="osk-key" on:click=move |_| on_key_inner(key_for_click.clone())>
                                        {key_str}
                                    </button>
                                }
                            }).collect_view()}
                            {if row_idx == 1 {
                                let on_key_bs = on_key;
                                Some(view! {
                                    <button class="osk-key osk-key-wide" on:click=move |_| on_key_bs("Backspace".into())>"←"</button>
                                })
                            } else {
                                None
                            }}
                        </div>
                    }
                }).collect_view()
            }}
            <div class="osk-row">
                {
                    let on_key_space = on_key;
                    let on_key_enter = on_key;
                    view! {
                        <button class="osk-key osk-key-space" on:click=move |_| on_key_space("Space".into())>{move || i18n.get().t("keyboard.space")}</button>
                        <button class="osk-key osk-key-wide" on:click=move |_| on_key_enter("Enter".into())>{move || i18n.get().t("keyboard.enter")}</button>
                    }
                }
            </div>
        </div>
    }
}

/// A compact numeric keypad for numerical fields (price, stock quantity, …).
/// Emits the same key strings as [`OnScreenKeyboard`] (`"0".."9"`, `"."`,
/// `"Backspace"`, `"Enter"`) so the existing key handlers can be reused. Like
/// the full keyboard, it is hidden on mobile via the `.osk` CSS rule, where the
/// native numeric keypad is used instead.
#[component]
pub fn NumericKeyboard(
    on_key: impl Fn(String) + Copy + Send + 'static,
    i18n: RwSignal<I18n>,
) -> impl IntoView {
    Effect::new(move |_| { scroll_osk_into_view(); });

    let rows = vec![vec!["7", "8", "9"], vec!["4", "5", "6"], vec!["1", "2", "3"], vec![".", "0", "Backspace"]];

    view! {
        <div class="osk osk-numeric">
            {rows.into_iter().map(|row| {
                view! {
                    <div class="osk-row">
                        {row.into_iter().map(|key| {
                            let on_key_inner = on_key;
                            let label = if key == "Backspace" { "←".to_string() } else { key.to_string() };
                            let value = key.to_string();
                            view! {
                                <button class="osk-key" on:click=move |_| on_key_inner(value.clone())>{label}</button>
                            }
                        }).collect_view()}
                    </div>
                }
            }).collect_view()}
            <div class="osk-row">
                {
                    let on_key_enter = on_key;
                    view! {
                        <button class="osk-key osk-key-space" on:click=move |_| on_key_enter("Enter".into())>{move || i18n.get().t("keyboard.enter")}</button>
                    }
                }
            </div>
        </div>
    }
}
