use leptos::prelude::*;

use crate::i18n::{available_currencies, available_languages, I18n};
use crate::models::*;
use crate::server_fns::*;

fn redirect_to_login() {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = web_sys::window().unwrap().location().set_href("/login");
    }
}

#[component]
pub fn AdminPage() -> impl IntoView {
    let i18n = expect_context::<RwSignal<I18n>>();
    let (authorized, set_authorized) = signal(false);
    let (users, set_users) = signal(Vec::<UserInfo>::new());
    let (error, set_error) = signal(Option::<String>::None);

    // Editing state
    let (editing_user, set_editing_user) = signal(Option::<UserInfo>::None);
    let (edit_name, set_edit_name) = signal(String::new());
    let (edit_role, set_edit_role) = signal(String::new());
    let (edit_pin, set_edit_pin) = signal(String::new());

    // Creating state
    let (creating, set_creating) = signal(false);
    let (new_name, set_new_name) = signal(String::new());
    let (new_role, set_new_role) = signal("cashier".to_string());
    let (new_pin, set_new_pin) = signal(String::new());

    // Keyboard target: "new_name" or "edit_name"
    let (kb_target, set_kb_target) = signal(Option::<String>::None);
    let (kb_shift, set_kb_shift) = signal(false);

    // Delete confirmation
    let (deleting_user, set_deleting_user) = signal(Option::<UserInfo>::None);

    // Auth check
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match get_current_user().await {
                Ok(Some(u)) if u.role == "admin" => {
                    set_authorized.set(true);
                }
                _ => redirect_to_login(),
            }
        });
    });

    let load_users = move || {
        leptos::task::spawn_local(async move {
            if let Ok(u) = fetch_user_list().await {
                set_users.set(u);
            }
        });
    };

    // Load users on mount
    Effect::new(move || {
        if authorized.get() {
            load_users();
        }
    });

    let start_edit = move |user: UserInfo| {
        set_edit_name.set(user.username.clone());
        set_edit_role.set(user.role.clone());
        set_edit_pin.set(String::new());
        set_editing_user.set(Some(user));
        set_creating.set(false);
        set_kb_target.set(None);
        set_error.set(None);
    };

    let cancel_edit = move |_| {
        set_editing_user.set(None);
        set_kb_target.set(None);
        set_error.set(None);
    };

    let save_edit = move |_| {
        let user = editing_user.get();
        if let Some(user) = user {
            let name = edit_name.get();
            let role = edit_role.get();
            let pin_val = edit_pin.get();
            let pin_opt = if pin_val.is_empty() { None } else { Some(pin_val) };
            let name_opt = if name == user.username {
                None
            } else {
                Some(name)
            };
            let role_opt = if role == user.role {
                None
            } else {
                Some(role)
            };
            set_error.set(None);
            leptos::task::spawn_local(async move {
                match update_user_account(user.id, name_opt, pin_opt, role_opt).await {
                    Ok(_) => {
                        set_editing_user.set(None);
                        set_kb_target.set(None);
                        load_users();
                    }
                    Err(e) => {
                        set_error.set(Some(
                            e.to_string()
                                .replace("error running server function: ", ""),
                        ));
                    }
                }
            });
        }
    };

    let start_create = move |_| {
        set_creating.set(true);
        set_editing_user.set(None);
        set_new_name.set(String::new());
        set_new_role.set("cashier".to_string());
        set_new_pin.set(String::new());
        set_kb_target.set(None);
        set_error.set(None);
    };

    let cancel_create = move |_| {
        set_creating.set(false);
        set_kb_target.set(None);
        set_error.set(None);
    };

    let save_create = move |_| {
        let name = new_name.get();
        let role = new_role.get();
        let pin_val = new_pin.get();
        if name.is_empty() || pin_val.len() < 4 {
            set_error.set(Some(i18n.get().t("admin.name_pin_required")));
            return;
        }
        set_error.set(None);
        leptos::task::spawn_local(async move {
            match create_user_account(name, pin_val, role).await {
                Ok(_) => {
                    set_creating.set(false);
                    set_kb_target.set(None);
                    load_users();
                }
                Err(e) => {
                    set_error.set(Some(
                        e.to_string()
                            .replace("error running server function: ", ""),
                    ));
                }
            }
        });
    };

    let confirm_delete = move |user: UserInfo| {
        set_deleting_user.set(Some(user));
    };

    let do_delete = move |_| {
        if let Some(user) = deleting_user.get() {
            leptos::task::spawn_local(async move {
                match delete_user_account(user.id).await {
                    Ok(_) => {
                        set_deleting_user.set(None);
                        load_users();
                    }
                    Err(e) => {
                        set_error.set(Some(
                            e.to_string()
                                .replace("error running server function: ", ""),
                        ));
                        set_deleting_user.set(None);
                    }
                }
            });
        }
    };

    let cancel_delete = move |_| {
        set_deleting_user.set(None);
    };

    // On-screen keyboard key handler
    let on_kb_key = move |key: String| {
        let target = kb_target.get();
        let Some(target) = target else { return };

        match key.as_str() {
            "Backspace" => {
                if target == "new_name" {
                    set_new_name.update(|s| { s.pop(); });
                } else {
                    set_edit_name.update(|s| { s.pop(); });
                }
            }
            "Enter" => {
                set_kb_target.set(None);
            }
            "Shift" => {
                set_kb_shift.update(|s| *s = !*s);
            }
            "Space" => {
                if target == "new_name" {
                    set_new_name.update(|s| s.push(' '));
                } else {
                    set_edit_name.update(|s| s.push(' '));
                }
            }
            ch => {
                let ch = if kb_shift.get() {
                    ch.to_uppercase()
                } else {
                    ch.to_lowercase()
                };
                if target == "new_name" {
                    set_new_name.update(|s| s.push_str(&ch));
                } else {
                    set_edit_name.update(|s| s.push_str(&ch));
                }
            }
        }
    };

    // PIN pad key handler for create/edit
    let on_pin_key = move |digit: &str, target: &str| {
        if target == "new" {
            set_new_pin.update(|p| {
                if p.len() < 8 {
                    p.push_str(digit);
                }
            });
        } else {
            set_edit_pin.update(|p| {
                if p.len() < 8 {
                    p.push_str(digit);
                }
            });
        }
    };

    let pin_clear = move |target: &str| {
        if target == "new" {
            set_new_pin.set(String::new());
        } else {
            set_edit_pin.set(String::new());
        }
    };

    view! {
        <Show when=move || authorized.get() fallback=move || view! { <div class="loading">{move || i18n.get().t("general.loading")}</div> }>

        // Delete confirmation modal
        <Show when=move || deleting_user.get().is_some() fallback=|| ()>
            {move || deleting_user.get().map(|u| {
                let username = u.username.clone();
                let confirm_msg = i18n.get().t("admin.confirm_delete_user").replace("{name}", &username);
                view! {
                    <div class="modal-overlay">
                        <div class="confirmation-modal">
                            <h3>{move || i18n.get().t("admin.delete_user")}</h3>
                            <p>{confirm_msg}</p>
                            <p class="warning-text">{move || i18n.get().t("general.cannot_undo")}</p>
                            <div class="modal-actions">
                                <button class="btn-danger" on:click=do_delete>{move || i18n.get().t("general.delete")}</button>
                                <button class="btn-secondary" on:click=cancel_delete>{move || i18n.get().t("general.cancel")}</button>
                            </div>
                        </div>
                    </div>
                }
            })}
        </Show>

        <div class="admin-page">
            <h2>{move || i18n.get().t("admin.user_management")}</h2>

            <Show when=move || error.get().is_some() fallback=|| ()>
                <div class="admin-error">{move || error.get().unwrap_or_default()}</div>
            </Show>

            // User list
            <Show when=move || !creating.get() && editing_user.get().is_none() fallback=|| ()>
                <div class="admin-user-list">
                    <For each=move || users.get() key=|u| u.id let:user>
                        {
                            let user_edit = user.clone();
                            let user_del = user.clone();
                            let role_key = match user.role.as_str() {
                                "admin" => "role.admin",
                                "cashier" => "role.cashier",
                                "cook" => "role.cook",
                                _ => "general.unknown",
                            };
                            let role_label = i18n.get().t(role_key);
                            view! {
                                <div class="admin-user-row">
                                    <div class="admin-user-info">
                                        <span class="admin-user-name">{user.username.clone()}</span>
                                        <span class="admin-user-role">{role_label}</span>
                                    </div>
                                    <div class="admin-user-actions">
                                        <button class="btn-primary-small" on:click=move |_| start_edit(user_edit.clone())>{move || i18n.get().t("general.edit")}</button>
                                        <button class="btn-danger-small" on:click=move |_| confirm_delete(user_del.clone())>{move || i18n.get().t("general.delete")}</button>
                                    </div>
                                </div>
                            }
                        }
                    </For>
                </div>
                <button class="btn-primary admin-add-btn" on:click=start_create>{move || i18n.get().t("admin.add_user")}</button>
            </Show>

            // Create user form
            <Show when=move || creating.get() fallback=|| ()>
                <div class="admin-form">
                    <h3>{move || i18n.get().t("admin.create_user")}</h3>

                    <div class="admin-form-field">
                        <label>{move || i18n.get().t("admin.username")}</label>
                        <div class="admin-input-row">
                            <input type="text" readonly value=move || new_name.get() placeholder=move || i18n.get().t("admin.tap_keyboard") />
                            <button class="btn-secondary-small" on:click=move |_| {
                                if kb_target.get() == Some("new_name".into()) {
                                    set_kb_target.set(None);
                                } else {
                                    set_kb_target.set(Some("new_name".into()));
                                }
                            }>{move || if kb_target.get() == Some("new_name".into()) { i18n.get().t("admin.hide_kb") } else { i18n.get().t("admin.keyboard") }}</button>
                        </div>
                    </div>

                    <Show when=move || kb_target.get() == Some("new_name".into()) fallback=|| ()>
                        <OnScreenKeyboard on_key=on_kb_key.clone() shift=kb_shift i18n=i18n />
                    </Show>

                    <div class="admin-form-field">
                        <label>{move || i18n.get().t("admin.role")}</label>
                        <div class="admin-role-buttons">
                            <button
                                class=move || if new_role.get() == "admin" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_new_role.set("admin".into())
                            >{move || i18n.get().t("role.admin")}</button>
                            <button
                                class=move || if new_role.get() == "cashier" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_new_role.set("cashier".into())
                            >{move || i18n.get().t("role.cashier")}</button>
                            <button
                                class=move || if new_role.get() == "cook" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_new_role.set("cook".into())
                            >{move || i18n.get().t("role.cook")}</button>
                        </div>
                    </div>

                    <div class="admin-form-field">
                        <label>{move || i18n.get().t("admin.pin")}</label>
                        <div class="pin-display pin-display-small">
                            {move || {
                                let len = new_pin.get().len();
                                (0..len).map(|_| view! { <span class="pin-dot">"*"</span> }).collect_view()
                            }}
                        </div>
                        <div class="pin-pad pin-pad-small">
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("1", "new")>"1"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("2", "new")>"2"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("3", "new")>"3"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("4", "new")>"4"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("5", "new")>"5"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("6", "new")>"6"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("7", "new")>"7"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("8", "new")>"8"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("9", "new")>"9"</button>
                            <button class="pin-btn pin-btn-sm pin-btn-clear" on:click=move |_| pin_clear("new")>{move || i18n.get().t("login.clear")}</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("0", "new")>"0"</button>
                            <button class="pin-btn pin-btn-sm pin-btn-ok" disabled=true>""</button>
                        </div>
                    </div>

                    <div class="admin-form-actions">
                        <button class="btn-primary" on:click=save_create>{move || i18n.get().t("admin.create")}</button>
                        <button class="btn-secondary" on:click=cancel_create>{move || i18n.get().t("general.cancel")}</button>
                    </div>
                </div>
            </Show>

            // Edit user form
            <Show when=move || editing_user.get().is_some() fallback=|| ()>
                <div class="admin-form">
                    <h3>{move || i18n.get().t("admin.edit_user")}</h3>

                    <div class="admin-form-field">
                        <label>{move || i18n.get().t("admin.username")}</label>
                        <div class="admin-input-row">
                            <input type="text" readonly value=move || edit_name.get() placeholder=move || i18n.get().t("admin.tap_keyboard") />
                            <button class="btn-secondary-small" on:click=move |_| {
                                if kb_target.get() == Some("edit_name".into()) {
                                    set_kb_target.set(None);
                                } else {
                                    set_kb_target.set(Some("edit_name".into()));
                                }
                            }>{move || if kb_target.get() == Some("edit_name".into()) { i18n.get().t("admin.hide_kb") } else { i18n.get().t("admin.keyboard") }}</button>
                        </div>
                    </div>

                    <Show when=move || kb_target.get() == Some("edit_name".into()) fallback=|| ()>
                        <OnScreenKeyboard on_key=on_kb_key.clone() shift=kb_shift i18n=i18n />
                    </Show>

                    <div class="admin-form-field">
                        <label>{move || i18n.get().t("admin.role")}</label>
                        <div class="admin-role-buttons">
                            <button
                                class=move || if edit_role.get() == "admin" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_edit_role.set("admin".into())
                            >{move || i18n.get().t("role.admin")}</button>
                            <button
                                class=move || if edit_role.get() == "cashier" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_edit_role.set("cashier".into())
                            >{move || i18n.get().t("role.cashier")}</button>
                            <button
                                class=move || if edit_role.get() == "cook" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_edit_role.set("cook".into())
                            >{move || i18n.get().t("role.cook")}</button>
                        </div>
                    </div>

                    <div class="admin-form-field">
                        <label>{move || i18n.get().t("admin.new_pin_hint")}</label>
                        <div class="pin-display pin-display-small">
                            {move || {
                                let len = edit_pin.get().len();
                                (0..len).map(|_| view! { <span class="pin-dot">"*"</span> }).collect_view()
                            }}
                        </div>
                        <div class="pin-pad pin-pad-small">
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("1", "edit")>"1"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("2", "edit")>"2"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("3", "edit")>"3"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("4", "edit")>"4"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("5", "edit")>"5"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("6", "edit")>"6"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("7", "edit")>"7"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("8", "edit")>"8"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("9", "edit")>"9"</button>
                            <button class="pin-btn pin-btn-sm pin-btn-clear" on:click=move |_| pin_clear("edit")>{move || i18n.get().t("login.clear")}</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("0", "edit")>"0"</button>
                            <button class="pin-btn pin-btn-sm pin-btn-ok" disabled=true>""</button>
                        </div>
                    </div>

                    <div class="admin-form-actions">
                        <button class="btn-primary" on:click=save_edit>{move || i18n.get().t("general.save")}</button>
                        <button class="btn-secondary" on:click=cancel_edit>{move || i18n.get().t("general.cancel")}</button>
                    </div>
                </div>
            </Show>
        </div>

        // Language setting section
        <div class="admin-page" style="margin-top: 2rem;">
            <h2>{move || i18n.get().t("admin.language_setting")}</h2>
            <div class="language-grid">
                {available_languages().into_iter().map(|(code, name)| {
                    let code_str = code.to_string();
                    let code_for_class = code.to_string();
                    view! {
                        <button
                            class=move || {
                                if i18n.get().lang == code_for_class {
                                    "language-btn language-btn-active"
                                } else {
                                    "language-btn"
                                }
                            }
                            on:click=move |_| {
                                let lang = code_str.clone();
                                leptos::task::spawn_local(async move {
                                    let _ = set_language_admin(lang).await;
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let _ = web_sys::window().unwrap().location().reload();
                                    }
                                });
                            }
                        >
                            <span class="language-name">{name}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>

        // Currency setting section
        <CurrencySettings i18n=i18n />

        // Remote printer passphrase setting
        <PrinterPassphraseSettings i18n=i18n />

        </Show>
    }
}

#[component]
fn CurrencySettings(i18n: RwSignal<I18n>) -> impl IntoView {
    let currency_ctx = expect_context::<RwSignal<String>>();
    let (custom_input, set_custom_input) = signal(String::new());

    let set_currency = move |sym: String| {
        let sym_clone = sym.clone();
        currency_ctx.set(sym);
        leptos::task::spawn_local(async move {
            let _ = set_currency_admin(sym_clone).await;
        });
    };

    let submit_custom = move |_| {
        let val = custom_input.get().trim().to_string();
        if !val.is_empty() {
            set_currency(val);
            set_custom_input.set(String::new());
        }
    };

    view! {
        <div class="admin-page" style="margin-top: 2rem;">
            <h2>{move || i18n.get().t("currency.setting")}</h2>
            <div class="currency-grid">
                {available_currencies().into_iter().map(|(sym, label)| {
                    let sym_str = sym.to_string();
                    let sym_for_class = sym.to_string();
                    view! {
                        <button
                            class=move || if currency_ctx.get() == sym_for_class { "currency-btn currency-btn-active" } else { "currency-btn" }
                            on:click=move |_| set_currency(sym_str.clone())
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>

            <div class="currency-custom">
                <label>{move || i18n.get().t("currency.custom")}</label>
                <div class="currency-custom-row">
                    <input
                        type="text"
                        maxlength="5"
                        placeholder=move || i18n.get().t("currency.custom_placeholder")
                        on:input=move |ev| set_custom_input.set(event_target_value(&ev))
                        prop:value=move || custom_input.get()
                    />
                    <button class="btn-primary" on:click=submit_custom>
                        {move || i18n.get().t("currency.set")}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn PrinterPassphraseSettings(i18n: RwSignal<I18n>) -> impl IntoView {
    let (passphrase_set, set_passphrase_set) = signal(false);
    let (input_value, set_input_value) = signal(String::new());
    let (status_msg, set_status_msg) = signal(Option::<String>::None);

    // Check if passphrase is currently configured
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            if let Ok(is_set) = get_printer_passphrase_set().await {
                set_passphrase_set.set(is_set);
            }
        });
    });

    let submit_passphrase = move |_| {
        let val = input_value.get().trim().to_string();
        if val.len() < 8 {
            set_status_msg.set(Some(i18n.get().t("admin.printer_passphrase_too_short")));
            return;
        }
        leptos::task::spawn_local(async move {
            match set_printer_passphrase(val).await {
                Ok(()) => {
                    set_passphrase_set.set(true);
                    set_input_value.set(String::new());
                    set_status_msg.set(Some(i18n.get().t("admin.printer_passphrase_updated")));
                }
                Err(e) => {
                    set_status_msg.set(Some(format!("{}", e)));
                }
            }
        });
    };

    let clear_passphrase = move |_| {
        leptos::task::spawn_local(async move {
            match clear_printer_passphrase().await {
                Ok(()) => {
                    set_passphrase_set.set(false);
                    set_status_msg.set(Some(i18n.get().t("admin.printer_passphrase_cleared")));
                }
                Err(e) => {
                    set_status_msg.set(Some(format!("{}", e)));
                }
            }
        });
    };

    view! {
        <div class="admin-page" style="margin-top: 2rem;">
            <h2>{move || i18n.get().t("admin.printer_settings")}</h2>
            <p>
                {move || if passphrase_set.get() {
                    i18n.get().t("admin.printer_passphrase_set")
                } else {
                    i18n.get().t("admin.printer_passphrase_not_set")
                }}
            </p>

            <div class="currency-custom">
                <label>{move || i18n.get().t("admin.printer_passphrase")}</label>
                <div class="currency-custom-row">
                    <input
                        type="password"
                        placeholder=move || i18n.get().t("admin.printer_passphrase_hint")
                        on:input=move |ev| set_input_value.set(event_target_value(&ev))
                        prop:value=move || input_value.get()
                    />
                    <button class="btn-primary" on:click=submit_passphrase>
                        {move || i18n.get().t("admin.printer_set_passphrase")}
                    </button>
                </div>
            </div>

            <Show when=move || passphrase_set.get() fallback=|| ()>
                <button
                    class="btn-primary"
                    style="margin-top: 0.5rem; background: #c0392b;"
                    on:click=clear_passphrase
                >
                    {move || i18n.get().t("admin.printer_clear_passphrase")}
                </button>
            </Show>

            <Show when=move || status_msg.get().is_some() fallback=|| ()>
                <p style="margin-top: 0.5rem; color: #27ae60; font-weight: bold;">
                    {move || status_msg.get().unwrap_or_default()}
                </p>
            </Show>
        </div>
    }
}

#[component]
fn OnScreenKeyboard(
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
