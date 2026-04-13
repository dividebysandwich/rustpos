use leptos::prelude::*;

use crate::i18n::I18n;

#[component]
pub fn OnScreenKeyboard(
    on_key: impl Fn(String) + Copy + Send + 'static,
    shift: ReadSignal<bool>,
    i18n: RwSignal<I18n>,
) -> impl IntoView {
    let rows_lower = vec![
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
        vec!["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
        vec!["a", "s", "d", "f", "g", "h", "j", "k", "l"],
        vec!["z", "x", "c", "v", "b", "n", "m"],
    ];
    let rows_upper = vec![
        vec!["!", "@", "#", "$", "%", "^", "&", "*", "(", ")"],
        vec!["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
        vec!["A", "S", "D", "F", "G", "H", "J", "K", "L"],
        vec!["Z", "X", "C", "V", "B", "N", "M"],
    ];

    view! {
        <div class="osk">
            {move || {
                let rows = if shift.get() { &rows_upper } else { &rows_lower };
                rows.iter().enumerate().map(|(row_idx, row)| {
                    view! {
                        <div class="osk-row">
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
                            {if row_idx == 3 {
                                let on_key_bs = on_key;
                                Some(view! {
                                    <button class="osk-key osk-key-wide" on:click=move |_| on_key_bs("Backspace".into())>{move || i18n.get().t("keyboard.bksp")}</button>
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
