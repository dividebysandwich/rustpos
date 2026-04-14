use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    hooks::use_location,
    StaticSegment,
};

use crate::i18n::I18n;
use crate::models::UserInfo;
use crate::pages::*;
use crate::server_fns::*;

#[derive(Clone, Copy)]
pub struct ActiveSaleView(pub RwSignal<String>);

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (dark_mode, set_dark_mode) = signal(false);
    let (current_user, set_current_user) = signal(Option::<UserInfo>::None);
    let i18n = RwSignal::new(I18n::new("en"));
    let currency = RwSignal::new("\u{20ac}".to_string());
    provide_context(i18n);
    provide_context(currency);
    provide_context(ActiveSaleView(RwSignal::new("sale".to_string())));

    Effect::new(move || {
        if dark_mode.get() {
            document()
                .document_element()
                .map(|el| el.set_attribute("data-theme", "dark"));
        } else {
            document()
                .document_element()
                .map(|el| el.remove_attribute("data-theme"));
        }
    });

    // Fetch language, currency + current user
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            if let Ok(Some(lang)) = get_config_language().await {
                i18n.set(I18n::new(&lang));
            }
            if let Ok(Some(cur)) = get_config_currency().await {
                currency.set(cur);
            }
            if let Ok(Some(u)) = get_current_user().await {
                set_current_user.set(Some(u));
            }
        });
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/rustpos.css"/>
        <Title text="RustPOS"/>

        <Router>
            <AppNavbar dark_mode set_dark_mode current_user />

            <main class="container">
                <Routes fallback=move || {
                    let i = i18n.get();
                    view! { <p>{i.t("general.page_not_found")}</p> }
                }>
                    <Route path=StaticSegment("") view=SalePage/>
                    <Route path=StaticSegment("transactions") view=TransactionsPage/>
                    <Route path=StaticSegment("items") view=ItemsPage/>
                    <Route path=StaticSegment("categories") view=CategoriesPage/>
                    <Route path=StaticSegment("reports") view=ReportsPage/>
                    <Route path=StaticSegment("kitchen") view=KitchenPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <Route path=StaticSegment("admin") view=AdminPage/>
                    <Route path=StaticSegment("setup") view=SetupPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn AppNavbar(
    dark_mode: ReadSignal<bool>,
    set_dark_mode: WriteSignal<bool>,
    current_user: ReadSignal<Option<UserInfo>>,
) -> impl IntoView {
    let i18n = expect_context::<RwSignal<I18n>>();
    let active_sale_view = expect_context::<ActiveSaleView>().0;
    let location = use_location();
    let (menu_open, set_menu_open) = signal(false);

    let is_kitchen = move || location.pathname.get().starts_with("/kitchen");
    let is_login = move || location.pathname.get().starts_with("/login");
    let is_setup = move || location.pathname.get().starts_with("/setup");

    let do_logout = move |_| {
        leptos::task::spawn_local(async move {
            let _ = logout().await;
            #[cfg(target_arch = "wasm32")]
            {
                let _ = web_sys::window().unwrap().location().set_href("/login");
            }
        });
    };

    view! {
        <Show when=move || !is_kitchen() && !is_login() && !is_setup() fallback=|| ()>
            <nav class="navbar">
                <div class="nav-container">
                    <img class="sitelogo" src="/logo_site.png"/>
                    <button class="hamburger-btn"
                        on:click=move |_| set_menu_open.update(|v| *v = !*v)
                        aria-label="Menu"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <line x1="3" y1="6" x2="21" y2="6"/>
                            <line x1="3" y1="12" x2="21" y2="12"/>
                            <line x1="3" y1="18" x2="21" y2="18"/>
                        </svg>
                    </button>
                    <div class=move || if menu_open.get() { "nav-links nav-links-open" } else { "nav-links" }>
                        {move || {
                            let user = current_user.get();
                            let i = i18n.get();
                            let role = user.as_ref().map(|u| u.role.as_str()).unwrap_or("");
                            let is_admin = role == "admin";
                            let is_cashier_or_admin = role == "admin" || role == "cashier";
                            view! {
                                <a href="/"
                                    class=move || if location.pathname.get() == "/" && active_sale_view.get() == "sale" { "active" } else { "" }
                                    on:click=move |_| { active_sale_view.set("sale".to_string()); set_menu_open.set(false); }
                                >{i.t("nav.sale")}</a>
                                <Show when=move || is_cashier_or_admin fallback=|| ()>
                                    <a href="/"
                                        class=move || if location.pathname.get() == "/" && active_sale_view.get() == "kitchen" { "active" } else { "" }
                                        on:click=move |_| { active_sale_view.set("kitchen".to_string()); set_menu_open.set(false); }
                                    >{i18n.get().t("sale.kitchen")}</a>
                                </Show>
                                <Show when=move || is_admin fallback=|| ()>
                                    <a href="/transactions"
                                        class=move || if location.pathname.get().starts_with("/transactions") { "active" } else { "" }
                                        on:click=move |_| set_menu_open.set(false)
                                    >{i18n.get().t("nav.transactions")}</a>
                                    <a href="/items"
                                        class=move || if location.pathname.get().starts_with("/items") { "active" } else { "" }
                                        on:click=move |_| set_menu_open.set(false)
                                    >{i18n.get().t("nav.items")}</a>
                                    <a href="/categories"
                                        class=move || if location.pathname.get().starts_with("/categories") { "active" } else { "" }
                                        on:click=move |_| set_menu_open.set(false)
                                    >{i18n.get().t("nav.categories")}</a>
                                    <a href="/reports"
                                        class=move || if location.pathname.get().starts_with("/reports") { "active" } else { "" }
                                        on:click=move |_| set_menu_open.set(false)
                                    >{i18n.get().t("nav.reports")}</a>
                                    <a href="/admin"
                                        class=move || if location.pathname.get().starts_with("/admin") { "active" } else { "" }
                                        on:click=move |_| set_menu_open.set(false)
                                    >{i18n.get().t("nav.settings")}</a>
                                </Show>
                            }
                        }}
                        <div class="nav-actions">
                            <button
                                class="dark-mode-toggle"
                                on:click=move |_| set_dark_mode.set(!dark_mode.get())
                                title=move || if dark_mode.get() { i18n.get().t("nav.light_mode") } else { i18n.get().t("nav.dark_mode") }
                            >
                                <Show when=move || dark_mode.get() fallback=|| view! {
                                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
                                    </svg>
                                }>
                                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <circle cx="12" cy="12" r="5"/>
                                        <line x1="12" y1="1" x2="12" y2="3"/>
                                        <line x1="12" y1="21" x2="12" y2="23"/>
                                        <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
                                        <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
                                        <line x1="1" y1="12" x2="3" y2="12"/>
                                        <line x1="21" y1="12" x2="23" y2="12"/>
                                        <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
                                        <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
                                    </svg>
                                </Show>
                            </button>
                            <Show when=move || current_user.get().is_some() fallback=|| ()>
                                <button class="nav-logout-btn" on:click=do_logout>
                                    {move || i18n.get().t("sale.logout")}
                                </button>
                            </Show>
                        </div>
                    </div>
                </div>
            </nav>
        </Show>
    }
}
