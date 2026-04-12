use leptos::prelude::*;

use crate::i18n::{available_currencies, available_languages, default_currency_for_language, I18n};
use crate::models::*;
use crate::server_fns::*;

#[component]
pub fn SetupPage() -> impl IntoView {
    // Phases: loading -> language -> currency -> credentials
    let (phase, set_phase) = signal("loading".to_string());
    let (credentials, set_credentials) = signal(Option::<InitialCredentials>::None);
    let (i18n, set_i18n) = signal(I18n::new("en"));
    let (selected_lang, set_selected_lang) = signal(String::new());
    let (custom_currency, set_custom_currency) = signal(String::new());

    // Check system state on mount
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match check_system_initialized().await {
                Ok((has_lang, has_users)) => {
                    if has_lang && has_users {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = web_sys::window().unwrap().location().set_href("/login");
                        }
                    } else if !has_lang {
                        set_phase.set("language".to_string());
                    } else {
                        set_phase.set("language".to_string());
                    }
                }
                Err(_) => set_phase.set("language".to_string()),
            }
        });
    });

    let select_language = move |lang: String| {
        set_selected_lang.set(lang.clone());
        set_i18n.set(I18n::new(&lang));
        leptos::task::spawn_local(async move {
            let _ = set_config_language(lang).await;
        });
        set_phase.set("currency".to_string());
    };

    let select_currency = move |symbol: String| {
        let symbol_clone = symbol.clone();
        leptos::task::spawn_local(async move {
            if set_config_currency(symbol_clone).await.is_ok() {
                match initialize_admin().await {
                    Ok(creds) => {
                        set_credentials.set(Some(creds));
                        set_phase.set("credentials".to_string());
                    }
                    Err(_) => {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = web_sys::window().unwrap().location().set_href("/login");
                        }
                    }
                }
            }
        });
    };

    let submit_custom_currency = move |_| {
        let val = custom_currency.get().trim().to_string();
        if !val.is_empty() {
            select_currency(val);
        }
    };

    let go_to_login = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = web_sys::window().unwrap().location().set_href("/login");
        }
    };

    view! {
        <div class="login-page">
            <div class="login-container setup-wide">
                <div class="login-header">
                    <img class="login-logo" src="/logo_site.png" alt="RustPOS" />
                </div>

                // Language selection
                <Show when=move || phase.get() == "language" fallback=|| ()>
                    <div class="setup-screen">
                        <h2>"Select Language"</h2>
                        <div class="language-grid">
                            {available_languages().into_iter().map(|(code, name)| {
                                let code_str = code.to_string();
                                view! {
                                    <button class="language-btn" on:click=move |_| select_language(code_str.clone())>
                                        <span class="language-name">{name}</span>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                </Show>

                // Currency selection
                <Show when=move || phase.get() == "currency" fallback=|| ()>
                    <div class="setup-screen">
                        <h2>{move || i18n.get().t("currency.select")}</h2>

                        <div class="currency-grid">
                            // Default currency for chosen language, shown first highlighted
                            {move || {
                                let lang = selected_lang.get();
                                let default_sym = default_currency_for_language(&lang);
                                let currencies = available_currencies();
                                currencies.into_iter().map(|(sym, label)| {
                                    let sym_str = sym.to_string();
                                    let is_default = sym == default_sym;
                                    let btn_class = if is_default { "currency-btn currency-btn-default" } else { "currency-btn" };
                                    view! {
                                        <button class=btn_class on:click=move |_| select_currency(sym_str.clone())>
                                            {label}
                                        </button>
                                    }
                                }).collect_view()
                            }}
                        </div>

                        <div class="currency-custom">
                            <label>{move || i18n.get().t("currency.custom")}</label>
                            <div class="currency-custom-row">
                                <input
                                    type="text"
                                    maxlength="5"
                                    placeholder=move || i18n.get().t("currency.custom_placeholder")
                                    on:input=move |ev| set_custom_currency.set(event_target_value(&ev))
                                    value=move || custom_currency.get()
                                />
                                <button class="btn-primary" on:click=submit_custom_currency>
                                    {move || i18n.get().t("currency.set")}
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>

                // Credentials display
                <Show when=move || phase.get() == "credentials" fallback=|| ()>
                    <div class="setup-screen">
                        <h2>{move || i18n.get().t("setup.initial_setup")}</h2>
                        <p class="setup-info">{move || i18n.get().t("setup.admin_created")}</p>
                        {move || credentials.get().map(|creds| {
                            let i = i18n.get();
                            view! {
                                <div class="setup-credentials">
                                    <div class="setup-field">
                                        <span class="setup-label">{i.t("setup.username_label")}</span>
                                        <span class="setup-value">{creds.username.clone()}</span>
                                    </div>
                                    <div class="setup-field">
                                        <span class="setup-label">{i.t("setup.pin_label")}</span>
                                        <span class="setup-value">{creds.pin.clone()}</span>
                                    </div>
                                </div>
                            }
                        })}
                        <p class="setup-warning">{move || i18n.get().t("setup.change_pin")}</p>
                        <button class="btn-primary setup-continue" on:click=go_to_login>
                            {move || i18n.get().t("setup.continue_login")}
                        </button>
                    </div>
                </Show>

                // Loading
                <Show when=move || phase.get() == "loading" fallback=|| ()>
                    <div class="loading">"Loading..."</div>
                </Show>
            </div>
        </div>
    }
}
