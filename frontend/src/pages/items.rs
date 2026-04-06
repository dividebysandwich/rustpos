use leptos::prelude::*;
use uuid::Uuid;

use crate::models::*;
use crate::server_fns::*;

const CURRENCY_SYMBOL: &str = "€";

#[component]
pub fn ItemsPage() -> impl IntoView {
    let (items, set_items) = signal(Vec::<Item>::new());
    let (categories, set_categories) = signal(Vec::<Category>::new());
    let (editing_item, set_editing_item) = signal(Option::<Item>::None);
    let (creating_item, set_creating_item) = signal(false);
    let (deleting_item, set_deleting_item) = signal(Option::<(Uuid, String)>::None);

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (price, set_price) = signal(String::new());
    let (category_id, set_category_id) = signal(String::new());
    let (sku, set_sku) = signal(String::new());
    let (in_stock, set_in_stock) = signal(true);

    let load_data = move || {
        leptos::task::spawn_local(async move {
            if let Ok(items_data) = fetch_items().await { set_items.set(items_data); }
            if let Ok(cats) = fetch_categories().await { set_categories.set(cats); }
        });
    };

    Effect::new(load_data.clone());

    let start_edit = move |item: Item| {
        set_name.set(item.name.clone());
        set_description.set(item.description.clone().unwrap_or_default());
        set_price.set(item.price.to_string());
        set_category_id.set(item.category_id.to_string());
        set_sku.set(item.sku.clone().unwrap_or_default());
        set_in_stock.set(item.in_stock);
        set_editing_item.set(Some(item));
    };

    let save_item = move |_| {
        let editing = editing_item.get();
        let creating = creating_item.get();
        if let Ok(price_val) = price.get().parse::<f64>() {
            if let Ok(cat_id) = category_id.get().parse::<Uuid>() {
                if creating {
                    let n = name.get();
                    let d = Some(description.get()).filter(|s| !s.is_empty());
                    let s = Some(sku.get()).filter(|s| !s.is_empty());
                    let stock = Some(in_stock.get());
                    leptos::task::spawn_local(async move {
                        if create_item(n, d, price_val, cat_id, s, stock).await.is_ok() {
                            load_data();
                            set_creating_item.set(false);
                        }
                    });
                } else if let Some(item) = editing {
                    let n = Some(name.get());
                    let d = Some(description.get()).filter(|s| !s.is_empty());
                    let s = Some(sku.get()).filter(|s| !s.is_empty());
                    let stock = Some(in_stock.get());
                    let item_id = item.id;
                    leptos::task::spawn_local(async move {
                        if update_item(item_id, n, d, Some(price_val), Some(cat_id), s, stock).await.is_ok() {
                            load_data();
                            set_editing_item.set(None);
                        }
                    });
                }
            }
        }
    };

    let confirm_delete = move |id: Uuid, name: String| { set_deleting_item.set(Some((id, name))); };
    let delete_item_handler = move |_| {
        if let Some((id, _)) = deleting_item.get() {
            leptos::task::spawn_local(async move {
                if delete_item(id).await.is_ok() { load_data(); set_deleting_item.set(None); }
            });
        }
    };
    let cancel_delete = move |_| { set_deleting_item.set(None); };
    let cancel_edit = move |_| {
        set_editing_item.set(None); set_creating_item.set(false);
        set_name.set(String::new()); set_description.set(String::new());
        set_price.set(String::new()); set_category_id.set(String::new());
        set_sku.set(String::new()); set_in_stock.set(true);
    };
    let start_create = move |_| {
        set_name.set(String::new()); set_description.set(String::new());
        set_price.set(String::new());
        set_category_id.set(if let Some(cat) = categories.get().first() { cat.id.to_string() } else { String::new() });
        set_sku.set(String::new()); set_in_stock.set(true);
        set_creating_item.set(true); set_editing_item.set(None);
    };

    view! {
        <div>
            <div class="page-header">
                <h2>"Items"</h2>
                <button class="btn-primary" on:click=start_create>"Add New Item"</button>
            </div>

            <Show when=move || deleting_item.get().is_some() fallback=|| ()>
                {move || {
                    deleting_item.get().map(|(_, name)| {
                        view! {
                            <div class="modal-overlay">
                                <div class="confirmation-modal">
                                    <h3>"Confirm Delete"</h3>
                                    <p>"Are you sure you want to delete \""<strong>{name}</strong>"\"?"</p>
                                    <p class="warning-text">"This action cannot be undone."</p>
                                    <div class="modal-actions">
                                        <button class="btn-danger" on:click=delete_item_handler>"Delete"</button>
                                        <button class="btn-secondary" on:click=cancel_delete>"Cancel"</button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                }}
            </Show>

            <Show when=move || editing_item.get().is_some() || creating_item.get() fallback=|| ()>
                <div class="edit-form">
                    <h3>{move || if creating_item.get() { "Create New Item" } else { "Edit Item" }}</h3>
                    <div class="form-grid">
                        <div class="form-group">
                            <label>"Name"</label>
                            <input type="text" value=move || name.get() on:input=move |ev| set_name.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>"Price"</label>
                            <input type="number" step="0.01" value=move || price.get() on:input=move |ev| set_price.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>"Category"</label>
                            <select prop:value=move || category_id.get() on:change=move |ev| set_category_id.set(event_target_value(&ev))>
                                <For each=move || categories.get() key=|cat| cat.id let:cat>
                                    <option value={cat.id.to_string()}>{cat.name.clone()}</option>
                                </For>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>"SKU"</label>
                            <input type="text" value=move || sku.get() on:input=move |ev| set_sku.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>"Description"</label>
                            <input type="text" value=move || description.get() on:input=move |ev| set_description.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>
                                <input type="checkbox" checked=move || in_stock.get() on:change=move |ev| set_in_stock.set(event_target_checked(&ev)) />
                                " In Stock"
                            </label>
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn-success" on:click=save_item>"Save"</button>
                        <button class="btn-secondary" on:click=cancel_edit>"Cancel"</button>
                    </div>
                </div>
            </Show>

            <table class="data-table">
                <thead><tr><th>"Name"</th><th>"Price"</th><th>"Category"</th><th>"SKU"</th><th>"In Stock"</th><th></th></tr></thead>
                <tbody>
                    <For each=move || items.get() key=|i| i.id let:item>
                        {
                            let item_clone = item.clone();
                            let item_id = item.id;
                            let item_name = item.name.clone();
                            let category_name = categories.get().iter()
                                .find(|c| c.id == item.category_id)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| "Unknown".to_string());
                            view! {
                                <tr>
                                    <td>{item.name.clone()}</td>
                                    <td>{format!("{} {:.2}", CURRENCY_SYMBOL, item.price)}</td>
                                    <td>{category_name}</td>
                                    <td>{item.sku.clone().unwrap_or_else(|| "-".to_string())}</td>
                                    <td>{if item.in_stock { "✓" } else { "✗" }}</td>
                                    <td class="data-table-actions">
                                        <button class="btn-small" on:click=move |_| start_edit(item_clone.clone())>"Edit"</button>
                                        <button class="btn-small btn-danger" on:click=move |_| confirm_delete(item_id, item_name.clone())>"Delete"</button>
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
