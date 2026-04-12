use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{A, Route, Router, Routes},
    hooks::use_location,
    StaticSegment,
};

use crate::models::UserInfo;
use crate::pages::*;
use crate::server_fns::*;

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

    // Fetch current user for navbar
    Effect::new(move || {
        leptos::task::spawn_local(async move {
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
                <Routes fallback=|| "Page not found">
                    <Route path=StaticSegment("") view=SalePage/>
                    <Route path=StaticSegment("transactions") view=TransactionsPage/>
                    <Route path=StaticSegment("items") view=ItemsPage/>
                    <Route path=StaticSegment("categories") view=CategoriesPage/>
                    <Route path=StaticSegment("reports") view=ReportsPage/>
                    <Route path=StaticSegment("kitchen") view=KitchenPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <Route path=StaticSegment("admin") view=AdminPage/>
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
    let location = use_location();
    let is_kitchen = move || location.pathname.get().starts_with("/kitchen");
    let is_login = move || location.pathname.get().starts_with("/login");

    view! {
        <Show when=move || !is_kitchen() && !is_login() fallback=|| ()>
            <nav class="navbar">
                <div class="nav-container">
                    <img class="sitelogo" src="/logo_site.png"/>
                    <div class="nav-links">
                        {move || {
                            let user = current_user.get();
                            let role = user.as_ref().map(|u| u.role.as_str()).unwrap_or("");
                            let is_admin = role == "admin";
                            view! {
                                <A href="/">"Sale"</A>
                                <Show when=move || is_admin fallback=|| ()>
                                    <A href="/transactions">"Transactions"</A>
                                    <A href="/items">"Items"</A>
                                    <A href="/categories">"Categories"</A>
                                    <A href="/reports">"Reports"</A>
                                    <A href="/admin">"Users"</A>
                                </Show>
                            }
                        }}
                    </div>
                    <button
                        class="dark-mode-toggle"
                        on:click=move |_| set_dark_mode.set(!dark_mode.get())
                        title=move || if dark_mode.get() { "Switch to light mode" } else { "Switch to dark mode" }
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
                </div>
            </nav>
        </Show>
    }
}
