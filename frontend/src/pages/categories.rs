use leptos::prelude::*;
use uuid::Uuid;

use crate::i18n::I18n;
use crate::models::*;
use crate::server_fns::*;

#[component]
pub fn CategoriesPage() -> impl IntoView {
    let i18n = expect_context::<RwSignal<I18n>>();
    let (authorized, set_authorized) = signal(false);
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match get_current_user().await {
                Ok(Some(u)) if u.role == "admin" => set_authorized.set(true),
                _ => {
                    #[cfg(target_arch = "wasm32")]
                    { let _ = web_sys::window().unwrap().location().set_href("/login"); }
                }
            }
        });
    });

    let (categories, set_categories) = signal(Vec::<Category>::new());
    let (editing_category, set_editing_category) = signal(Option::<Category>::None);
    let (creating_category, set_creating_category) = signal(false);
    let (deleting_category, set_deleting_category) = signal(Option::<(Uuid, String)>::None);

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());

    // Trigger signal: incrementing this causes the Effect to re-run
    let (reload, set_reload) = signal(0u32);

    Effect::new(move || {
        reload.get(); // subscribe to trigger
        leptos::task::spawn_local(async move {
            if let Ok(cats) = fetch_categories().await { set_categories.set(cats); }
        });
    });

    let start_edit = move |category: Category| {
        set_name.set(category.name.clone());
        set_description.set(category.description.clone().unwrap_or_default());
        set_editing_category.set(Some(category));
    };

    let save_category = move |_| {
        let editing = editing_category.get();
        let creating = creating_category.get();
        if creating {
            let n = name.get();
            let d = Some(description.get()).filter(|s| !s.is_empty());
            leptos::task::spawn_local(async move {
                if create_category(n, d).await.is_ok() {
                    set_creating_category.set(false);
                    set_reload.update(|v| *v += 1);
                }
            });
        } else if let Some(category) = editing {
            let n = Some(name.get());
            let d = Some(description.get()).filter(|s| !s.is_empty());
            let cat_id = category.id;
            leptos::task::spawn_local(async move {
                if update_category(cat_id, n, d).await.is_ok() {
                    set_editing_category.set(None);
                    set_reload.update(|v| *v += 1);
                }
            });
        }
    };

    let confirm_delete = move |id: Uuid, name: String| { set_deleting_category.set(Some((id, name))); };
    let delete_category_handler = move |_| {
        if let Some((id, _)) = deleting_category.get() {
            leptos::task::spawn_local(async move {
                if delete_category(id).await.is_ok() {
                    set_deleting_category.set(None);
                    set_reload.update(|v| *v += 1);
                }
            });
        }
    };
    let cancel_delete = move |_| { set_deleting_category.set(None); };
    let cancel_edit = move |_| {
        set_editing_category.set(None); set_creating_category.set(false);
        set_name.set(String::new()); set_description.set(String::new());
    };
    let start_create = move |_| {
        set_name.set(String::new()); set_description.set(String::new());
        set_creating_category.set(true); set_editing_category.set(None);
    };

    view! {
        <Show when=move || authorized.get() fallback=move || view! { <div class="loading">{move || i18n.get().t("general.loading")}</div> }>
        <div>
            <div class="page-header">
                <h2>{move || i18n.get().t("categories.title")}</h2>
                <button class="btn-primary" on:click=start_create
                    disabled=move || editing_category.get().is_some() || creating_category.get()
                >{move || i18n.get().t("categories.add")}</button>
            </div>

            <Show when=move || deleting_category.get().is_some() fallback=|| ()>
                {move || {
                    deleting_category.get().map(|(_, cat_name)| {
                        let i = i18n.get();
                        let confirm_msg = i.t("categories.confirm_delete").replace("{name}", &cat_name);
                        view! {
                            <div class="modal-overlay">
                                <div class="confirmation-modal">
                                    <h3>{i.t("general.confirm_delete")}</h3>
                                    <p>{confirm_msg}</p>
                                    <p class="warning-text">{i.t("categories.delete_warning")}</p>
                                    <div class="modal-actions">
                                        <button class="btn-danger" on:click=delete_category_handler>{i.t("general.delete")}</button>
                                        <button class="btn-secondary" on:click=cancel_delete>{i.t("general.cancel")}</button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                }}
            </Show>

            <Show when=move || editing_category.get().is_some() || creating_category.get() fallback=|| ()>
                <div class="edit-form">
                    <h3>{move || if creating_category.get() { i18n.get().t("categories.create") } else { i18n.get().t("categories.edit") }}</h3>
                    <div class="form-grid">
                        <div class="form-group">
                            <label>{move || i18n.get().t("general.name")}</label>
                            <input type="text" value=move || name.get() on:input=move |ev| set_name.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>{move || i18n.get().t("general.description")}</label>
                            <input type="text" value=move || description.get() on:input=move |ev| set_description.set(event_target_value(&ev)) />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn-success" on:click=save_category>{move || i18n.get().t("general.save")}</button>
                        <button class="btn-secondary" on:click=cancel_edit>{move || i18n.get().t("general.cancel")}</button>
                    </div>
                </div>
            </Show>

            <table class="data-table">
                <thead><tr><th>{move || i18n.get().t("general.name")}</th><th>{move || i18n.get().t("general.description")}</th><th></th></tr></thead>
                <tbody>
                    <For each=move || categories.get() key=|c| (c.id, c.description.clone(), c.name.clone()) let:category>
                        {
                            let category_clone = category.clone();
                            let category_id = category.id;
                            let category_name = category.name.clone();
                            view! {
                                <tr>
                                    <td>{category.name.clone()}</td>
                                    <td>{category.description.clone().unwrap_or_else(|| "-".to_string())}</td>
                                    <td class="data-table-actions">
                                        <button class="btn-small" on:click=move |_| start_edit(category_clone.clone())
                                            disabled=move || editing_category.get().is_some() || creating_category.get()
                                        >{move || i18n.get().t("general.edit")}</button>
                                        <button class="btn-small btn-danger" on:click=move |_| confirm_delete(category_id, category_name.clone())
                                            disabled=move || editing_category.get().is_some() || creating_category.get()
                                        >{move || i18n.get().t("general.delete")}</button>
                                    </td>
                                </tr>
                            }
                        }
                    </For>
                </tbody>
            </table>
        </div>
        </Show>
    }
}
