use leptos::prelude::*;

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
            set_error.set(Some("Name required, PIN must be at least 4 digits".into()));
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
        <Show when=move || authorized.get() fallback=|| view! { <div class="loading">"Loading..."</div> }>

        // Delete confirmation modal
        <Show when=move || deleting_user.get().is_some() fallback=|| ()>
            {move || deleting_user.get().map(|u| view! {
                <div class="modal-overlay">
                    <div class="confirmation-modal">
                        <h3>"Delete User"</h3>
                        <p>"Are you sure you want to delete user \""{ u.username.clone() }"\"?"</p>
                        <p class="warning-text">"This action cannot be undone."</p>
                        <div class="modal-actions">
                            <button class="btn-danger" on:click=do_delete>"Delete"</button>
                            <button class="btn-secondary" on:click=cancel_delete>"Cancel"</button>
                        </div>
                    </div>
                </div>
            })}
        </Show>

        <div class="admin-page">
            <h2>"User Management"</h2>

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
                            let role_label = match user.role.as_str() {
                                "admin" => "Admin",
                                "cashier" => "Cashier",
                                "cook" => "Cook",
                                _ => "Unknown",
                            };
                            view! {
                                <div class="admin-user-row">
                                    <div class="admin-user-info">
                                        <span class="admin-user-name">{user.username.clone()}</span>
                                        <span class="admin-user-role">{role_label}</span>
                                    </div>
                                    <div class="admin-user-actions">
                                        <button class="btn-primary-small" on:click=move |_| start_edit(user_edit.clone())>"Edit"</button>
                                        <button class="btn-danger-small" on:click=move |_| confirm_delete(user_del.clone())>"Delete"</button>
                                    </div>
                                </div>
                            }
                        }
                    </For>
                </div>
                <button class="btn-primary admin-add-btn" on:click=start_create>"Add User"</button>
            </Show>

            // Create user form
            <Show when=move || creating.get() fallback=|| ()>
                <div class="admin-form">
                    <h3>"Create New User"</h3>

                    <div class="admin-form-field">
                        <label>"Username"</label>
                        <div class="admin-input-row">
                            <input type="text" readonly value=move || new_name.get() placeholder="Tap keyboard to type" />
                            <button class="btn-secondary-small" on:click=move |_| {
                                if kb_target.get() == Some("new_name".into()) {
                                    set_kb_target.set(None);
                                } else {
                                    set_kb_target.set(Some("new_name".into()));
                                }
                            }>{move || if kb_target.get() == Some("new_name".into()) { "Hide KB" } else { "Keyboard" }}</button>
                        </div>
                    </div>

                    <Show when=move || kb_target.get() == Some("new_name".into()) fallback=|| ()>
                        <OnScreenKeyboard on_key=on_kb_key.clone() shift=kb_shift />
                    </Show>

                    <div class="admin-form-field">
                        <label>"Role"</label>
                        <div class="admin-role-buttons">
                            <button
                                class=move || if new_role.get() == "admin" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_new_role.set("admin".into())
                            >"Admin"</button>
                            <button
                                class=move || if new_role.get() == "cashier" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_new_role.set("cashier".into())
                            >"Cashier"</button>
                            <button
                                class=move || if new_role.get() == "cook" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_new_role.set("cook".into())
                            >"Cook"</button>
                        </div>
                    </div>

                    <div class="admin-form-field">
                        <label>"PIN"</label>
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
                            <button class="pin-btn pin-btn-sm pin-btn-clear" on:click=move |_| pin_clear("new")>"Clear"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("0", "new")>"0"</button>
                            <button class="pin-btn pin-btn-sm pin-btn-ok" disabled=true>""</button>
                        </div>
                    </div>

                    <div class="admin-form-actions">
                        <button class="btn-primary" on:click=save_create>"Create"</button>
                        <button class="btn-secondary" on:click=cancel_create>"Cancel"</button>
                    </div>
                </div>
            </Show>

            // Edit user form
            <Show when=move || editing_user.get().is_some() fallback=|| ()>
                <div class="admin-form">
                    <h3>"Edit User"</h3>

                    <div class="admin-form-field">
                        <label>"Username"</label>
                        <div class="admin-input-row">
                            <input type="text" readonly value=move || edit_name.get() placeholder="Tap keyboard to type" />
                            <button class="btn-secondary-small" on:click=move |_| {
                                if kb_target.get() == Some("edit_name".into()) {
                                    set_kb_target.set(None);
                                } else {
                                    set_kb_target.set(Some("edit_name".into()));
                                }
                            }>{move || if kb_target.get() == Some("edit_name".into()) { "Hide KB" } else { "Keyboard" }}</button>
                        </div>
                    </div>

                    <Show when=move || kb_target.get() == Some("edit_name".into()) fallback=|| ()>
                        <OnScreenKeyboard on_key=on_kb_key.clone() shift=kb_shift />
                    </Show>

                    <div class="admin-form-field">
                        <label>"Role"</label>
                        <div class="admin-role-buttons">
                            <button
                                class=move || if edit_role.get() == "admin" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_edit_role.set("admin".into())
                            >"Admin"</button>
                            <button
                                class=move || if edit_role.get() == "cashier" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_edit_role.set("cashier".into())
                            >"Cashier"</button>
                            <button
                                class=move || if edit_role.get() == "cook" { "role-btn role-btn-active" } else { "role-btn" }
                                on:click=move |_| set_edit_role.set("cook".into())
                            >"Cook"</button>
                        </div>
                    </div>

                    <div class="admin-form-field">
                        <label>"New PIN (leave empty to keep current)"</label>
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
                            <button class="pin-btn pin-btn-sm pin-btn-clear" on:click=move |_| pin_clear("edit")>"Clear"</button>
                            <button class="pin-btn pin-btn-sm" on:click=move |_| on_pin_key("0", "edit")>"0"</button>
                            <button class="pin-btn pin-btn-sm pin-btn-ok" disabled=true>""</button>
                        </div>
                    </div>

                    <div class="admin-form-actions">
                        <button class="btn-primary" on:click=save_edit>"Save"</button>
                        <button class="btn-secondary" on:click=cancel_edit>"Cancel"</button>
                    </div>
                </div>
            </Show>
        </div>
        </Show>
    }
}

#[component]
fn OnScreenKeyboard(
    on_key: impl Fn(String) + Copy + Send + 'static,
    shift: ReadSignal<bool>,
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
                                    >"Shift"</button>
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
                                    <button class="osk-key osk-key-wide" on:click=move |_| on_key_bs("Backspace".into())>"Bksp"</button>
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
                        <button class="osk-key osk-key-space" on:click=move |_| on_key_space("Space".into())>"Space"</button>
                        <button class="osk-key osk-key-wide" on:click=move |_| on_key_enter("Enter".into())>"Enter"</button>
                    }
                }
            </div>
        </div>
    }
}
