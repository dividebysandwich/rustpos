use leptos::prelude::*;
use uuid::Uuid;

use crate::models::*;
use crate::server_fns::*;

#[component]
pub fn CategoriesPage() -> impl IntoView {
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
        <div>
            <div class="page-header">
                <h2>"Categories"</h2>
                <button class="btn-primary" on:click=start_create>"Add New Category"</button>
            </div>

            <Show when=move || deleting_category.get().is_some() fallback=|| ()>
                {move || {
                    deleting_category.get().map(|(_, name)| {
                        view! {
                            <div class="modal-overlay">
                                <div class="confirmation-modal">
                                    <h3>"Confirm Delete"</h3>
                                    <p>"Are you sure you want to delete the category \""<strong>{name}</strong>"\"?"</p>
                                    <p class="warning-text">"Warning: This will NOT delete items in this category, but they may become harder to find."</p>
                                    <div class="modal-actions">
                                        <button class="btn-danger" on:click=delete_category_handler>"Delete"</button>
                                        <button class="btn-secondary" on:click=cancel_delete>"Cancel"</button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                }}
            </Show>

            <Show when=move || editing_category.get().is_some() || creating_category.get() fallback=|| ()>
                <div class="edit-form">
                    <h3>{move || if creating_category.get() { "Create New Category" } else { "Edit Category" }}</h3>
                    <div class="form-grid">
                        <div class="form-group">
                            <label>"Name"</label>
                            <input type="text" value=move || name.get() on:input=move |ev| set_name.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>"Description"</label>
                            <input type="text" value=move || description.get() on:input=move |ev| set_description.set(event_target_value(&ev)) />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn-success" on:click=save_category>"Save"</button>
                        <button class="btn-secondary" on:click=cancel_edit>"Cancel"</button>
                    </div>
                </div>
            </Show>

            <table class="data-table">
                <thead><tr><th>"Name"</th><th>"Description"</th><th></th></tr></thead>
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
                                        <button class="btn-small" on:click=move |_| start_edit(category_clone.clone())>"Edit"</button>
                                        <button class="btn-small btn-danger" on:click=move |_| confirm_delete(category_id, category_name.clone())>"Delete"</button>
                                    </td>
                                </tr>
                            }
                        }
                    </For>
                </tbody>
            </table>
        </div>
    }
}
