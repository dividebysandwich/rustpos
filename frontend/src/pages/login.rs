use leptos::prelude::*;

use crate::models::*;
use crate::server_fns::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let (users, set_users) = signal(Vec::<UserInfo>::new());
    let (selected_user, set_selected_user) = signal(Option::<UserInfo>::None);
    let (pin, set_pin) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    // Initial setup state
    let (setup_creds, set_setup_creds) = signal(Option::<InitialCredentials>::None);
    let (setup_checked, set_setup_checked) = signal(false);

    // Check for initial setup, then load users
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            if let Ok(Some(creds)) = check_initial_setup().await {
                set_setup_creds.set(Some(creds));
            }
            set_setup_checked.set(true);
            if let Ok(u) = fetch_user_list().await {
                set_users.set(u);
            }
        });
    });

    let handle_acknowledge = move |_| {
        leptos::task::spawn_local(async move {
            let _ = acknowledge_setup().await;
            set_setup_creds.set(None);
        });
    };

    let handle_pin_digit = move |digit: &str| {
        set_error.set(None);
        set_pin.update(|p| {
            if p.len() < 8 {
                p.push_str(digit);
            }
        });
    };

    let handle_pin_clear = move |_| {
        set_pin.set(String::new());
        set_error.set(None);
    };

    let handle_back = move |_| {
        set_selected_user.set(None);
        set_pin.set(String::new());
        set_error.set(None);
    };

    let handle_login = move |_| {
        let current_pin = pin.get();
        let user = selected_user.get();
        if current_pin.is_empty() || user.is_none() {
            return;
        }
        let user = user.unwrap();
        set_loading.set(true);
        set_error.set(None);
        leptos::task::spawn_local(async move {
            match login(user.id, current_pin).await {
                Ok(logged_in) => {
                    let target = if logged_in.role == "cook" {
                        "/kitchen"
                    } else {
                        "/"
                    };
                    #[cfg(target_arch = "wasm32")]
                    {
                        let _ = web_sys::window().unwrap().location().set_href(target);
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = target;
                    }
                }
                Err(e) => {
                    set_error.set(Some(
                        e.to_string()
                            .replace("error running server function: ", ""),
                    ));
                    set_pin.set(String::new());
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="login-page">
            <div class="login-container">
                <div class="login-header">
                    <h1>"RustPOS"</h1>
                </div>

                // Initial setup screen
                <Show when=move || setup_creds.get().is_some() && setup_checked.get() fallback=move || view! {
                    // Normal login flow
                    <Show when=move || selected_user.get().is_none() fallback=move || {
                        // PIN entry screen
                        let user = selected_user.get().unwrap();
                        view! {
                            <div class="login-pin-screen">
                                <h2 class="login-welcome">"Welcome, "{user.username.clone()}</h2>
                                <p class="login-subtitle">"Enter your PIN"</p>

                                <div class="pin-display">
                                    {move || {
                                        let len = pin.get().len();
                                        (0..len).map(|_| view! { <span class="pin-dot">"*"</span> }).collect_view()
                                    }}
                                </div>

                                <Show when=move || error.get().is_some() fallback=|| ()>
                                    <div class="login-error">{move || error.get().unwrap_or_default()}</div>
                                </Show>

                                <div class="pin-pad">
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("1") disabled=loading>"1"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("2") disabled=loading>"2"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("3") disabled=loading>"3"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("4") disabled=loading>"4"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("5") disabled=loading>"5"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("6") disabled=loading>"6"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("7") disabled=loading>"7"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("8") disabled=loading>"8"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("9") disabled=loading>"9"</button>
                                    <button class="pin-btn pin-btn-clear" on:click=handle_pin_clear disabled=loading>"Clear"</button>
                                    <button class="pin-btn" on:click=move |_| handle_pin_digit("0") disabled=loading>"0"</button>
                                    <button class="pin-btn pin-btn-ok" on:click=handle_login disabled=move || loading.get() || pin.get().is_empty()>"OK"</button>
                                </div>

                                <button class="login-back-btn" on:click=handle_back>"Back"</button>
                            </div>
                        }
                    }>
                        // User selection screen
                        <div class="login-user-select">
                            <h2>"Select User"</h2>
                            <div class="user-grid">
                                <For each=move || users.get() key=|u| u.id let:user>
                                    {
                                        let user_clone = user.clone();
                                        let role_label = match user.role.as_str() {
                                            "admin" => "Admin",
                                            "cashier" => "Cashier",
                                            "cook" => "Cook",
                                            _ => "",
                                        };
                                        view! {
                                            <button class="user-select-btn" on:click=move |_| {
                                                set_selected_user.set(Some(user_clone.clone()));
                                                set_pin.set(String::new());
                                                set_error.set(None);
                                            }>
                                                <div class="user-select-icon">
                                                    {user.username.chars().next().unwrap_or('?').to_uppercase().to_string()}
                                                </div>
                                                <div class="user-select-name">{user.username.clone()}</div>
                                                <div class="user-select-role">{role_label}</div>
                                            </button>
                                        }
                                    }
                                </For>
                            </div>
                        </div>
                    </Show>
                }>
                    // Setup credentials display
                    {move || {
                        setup_creds.get().map(|creds| view! {
                            <div class="setup-screen">
                                <h2>"Initial Setup"</h2>
                                <p class="setup-info">"An admin account has been created. Please save these credentials:"</p>
                                <div class="setup-credentials">
                                    <div class="setup-field">
                                        <span class="setup-label">"Username:"</span>
                                        <span class="setup-value">{creds.username.clone()}</span>
                                    </div>
                                    <div class="setup-field">
                                        <span class="setup-label">"PIN:"</span>
                                        <span class="setup-value">{creds.pin.clone()}</span>
                                    </div>
                                </div>
                                <p class="setup-warning">"Please change your PIN after logging in."</p>
                                <button class="btn-primary setup-continue" on:click=handle_acknowledge>"Continue to Login"</button>
                            </div>
                        })
                    }}
                </Show>
            </div>
        </div>
    }
}
