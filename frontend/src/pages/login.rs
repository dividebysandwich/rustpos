use leptos::prelude::*;

use crate::i18n::I18n;
use crate::models::*;
use crate::server_fns::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let i18n = expect_context::<RwSignal<I18n>>();
    let (users, set_users) = signal(Vec::<UserInfo>::new());
    let (selected_user, set_selected_user) = signal(Option::<UserInfo>::None);
    let (pin, set_pin) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    // Check if system is initialized, redirect to setup if not
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match check_system_initialized().await {
                Ok((has_lang, has_users)) => {
                    if !has_lang || !has_users {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = web_sys::window().unwrap().location().set_href("/setup");
                        }
                        return;
                    }
                }
                Err(_) => {}
            }
            // Load language
            if let Ok(Some(lang)) = get_config_language().await {
                i18n.set(I18n::new(&lang));
            }
            if let Ok(u) = fetch_user_list().await {
                set_users.set(u);
            }
        });
    });

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
                    <img class="login-logo" src="/logo_site.png" alt="RustPOS" />
                </div>

                <Show when=move || selected_user.get().is_none() fallback=move || {
                    // PIN entry screen
                    let user = selected_user.get().unwrap();
                    let welcome = i18n.get().t("login.welcome").replace("{name}", &user.username);
                    view! {
                        <div class="login-pin-screen">
                            <h2 class="login-welcome">{welcome}</h2>
                            <p class="login-subtitle">{move || i18n.get().t("login.enter_pin")}</p>

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
                                <button class="pin-btn pin-btn-clear" on:click=handle_pin_clear disabled=loading>{move || i18n.get().t("login.clear")}</button>
                                <button class="pin-btn" on:click=move |_| handle_pin_digit("0") disabled=loading>"0"</button>
                                <button class="pin-btn pin-btn-ok" on:click=handle_login disabled=move || loading.get() || pin.get().is_empty()>{move || i18n.get().t("login.ok")}</button>
                            </div>

                            <button class="login-back-btn" on:click=handle_back>{move || i18n.get().t("login.back")}</button>
                        </div>
                    }
                }>
                    // User selection screen
                    <div class="login-user-select">
                        <h2>{move || i18n.get().t("login.select_user")}</h2>
                        <div class="user-grid">
                            <For each=move || users.get() key=|u| u.id let:user>
                                {
                                    let user_clone = user.clone();
                                    let role_key = match user.role.as_str() {
                                        "admin" => "role.admin",
                                        "cashier" => "role.cashier",
                                        "cook" => "role.cook",
                                        _ => "general.unknown",
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
                                            <div class="user-select-role">{move || i18n.get().t(role_key)}</div>
                                        </button>
                                    }
                                }
                            </For>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    }
}
