use leptos::prelude::*;

use crate::i18n::{available_languages, I18n};
use crate::models::*;
use crate::server_fns::*;

#[component]
pub fn SetupPage() -> impl IntoView {
    let (phase, set_phase) = signal("loading".to_string()); // loading, language, credentials
    let (credentials, set_credentials) = signal(Option::<InitialCredentials>::None);
    let (i18n, set_i18n) = signal(I18n::new("en"));

    // Check system state on mount
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match check_system_initialized().await {
                Ok((has_lang, has_users)) => {
                    if has_lang && has_users {
                        // Already set up, go to login
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = web_sys::window().unwrap().location().set_href("/login");
                        }
                    } else if !has_lang {
                        set_phase.set("language".to_string());
                    } else {
                        // Has language but no users — shouldn't happen, but handle it
                        set_phase.set("language".to_string());
                    }
                }
                Err(_) => set_phase.set("language".to_string()),
            }
        });
    });

    let select_language = move |lang: String| {
        let lang_clone = lang.clone();
        set_i18n.set(I18n::new(&lang));
        leptos::task::spawn_local(async move {
            if set_config_language(lang_clone).await.is_ok() {
                // Now create admin user
                match initialize_admin().await {
                    Ok(creds) => {
                        set_credentials.set(Some(creds));
                        set_phase.set("credentials".to_string());
                    }
                    Err(_) => {
                        // Admin might already exist, go to login
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = web_sys::window().unwrap().location().set_href("/login");
                        }
                    }
                }
            }
        });
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

                <Show when=move || phase.get() == "language" fallback=move || {
                    if phase.get() == "credentials" {
                        view! {
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
                        }.into_any()
                    } else {
                        view! { <div class="loading">"Loading..."</div> }.into_any()
                    }
                }>
                    <div class="setup-screen">
                        <h2>{move || i18n.get().t("setup.select_language")}</h2>
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
            </div>
        </div>
    }
}
