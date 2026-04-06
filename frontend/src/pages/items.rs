use leptos::prelude::*;
use uuid::Uuid;

use crate::models::*;
use crate::server_fns::*;

const CURRENCY_SYMBOL: &str = "€";

#[cfg(not(target_arch = "wasm32"))]
fn handle_image_file(_ev: leptos::ev::Event, _set: WriteSignal<Option<String>>) {}

#[cfg(target_arch = "wasm32")]
fn handle_image_file(ev: leptos::ev::Event, set_image_preview: WriteSignal<Option<String>>) {
    use wasm_bindgen::prelude::*;
    use web_sys::{FileReader, HtmlCanvasElement, HtmlImageElement};

    let input: web_sys::HtmlInputElement = event_target(&ev);
    let file = input.files().and_then(|f| f.get(0));
    let Some(file) = file else { return };

    let reader = FileReader::new().unwrap();
    let reader_clone = reader.clone();
    let closure = Closure::wrap(Box::new(move || {
        let Some(data_url) = reader_clone.result().ok().and_then(|v| v.as_string()) else { return };
        let data_url_clone = data_url.clone();
        let img = HtmlImageElement::new().unwrap();
        let img_clone = img.clone();
        let onload = Closure::wrap(Box::new(move || {
            let w = img_clone.natural_width();
            let h = img_clone.natural_height();
            let max_size = 200u32;
            let (nw, nh) = if w > h {
                (max_size, (max_size as f64 * h as f64 / w as f64) as u32)
            } else {
                ((max_size as f64 * w as f64 / h as f64) as u32, max_size)
            };
            let doc = leptos::prelude::document();
            let canvas: HtmlCanvasElement = doc.create_element("canvas").unwrap().unchecked_into();
            canvas.set_width(nw);
            canvas.set_height(nh);
            let ctx: web_sys::CanvasRenderingContext2d = canvas
                .get_context("2d").unwrap().unwrap().unchecked_into();
            let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(
                &img_clone, 0.0, 0.0, nw as f64, nh as f64,
            );
            let resized = canvas.to_data_url_with_type("image/webp")
                .or_else(|_| canvas.to_data_url_with_type("image/png"))
                .unwrap_or_default();
            set_image_preview.set(Some(resized));
        }) as Box<dyn Fn()>);
        img.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        img.set_src(&data_url_clone);
    }) as Box<dyn Fn()>);
    reader.set_onload(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    let _ = reader.read_as_data_url(&file);
}

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
    let (image_preview, set_image_preview) = signal(Option::<String>::None);
    let (track_stock, set_track_stock) = signal(false);
    let (stock_quantity, set_stock_quantity) = signal(String::new());
    let (kitchen_item, set_kitchen_item) = signal(false);

    let (reload, set_reload) = signal(0u32);

    Effect::new(move || {
        reload.get();
        leptos::task::spawn_local(async move {
            if let Ok(items_data) = fetch_items().await { set_items.set(items_data); }
            if let Ok(cats) = fetch_categories().await { set_categories.set(cats); }
        });
    });

    let start_edit = move |item: Item| {
        set_name.set(item.name.clone());
        set_description.set(item.description.clone().unwrap_or_default());
        set_price.set(item.price.to_string());
        set_category_id.set(item.category_id.to_string());
        set_sku.set(item.sku.clone().unwrap_or_default());
        set_in_stock.set(item.in_stock);
        set_image_preview.set(item.image_path.clone());
        set_track_stock.set(item.stock_quantity.is_some());
        set_stock_quantity.set(item.stock_quantity.map(|q| q.to_string()).unwrap_or_default());
        set_kitchen_item.set(item.kitchen_item);
        set_editing_item.set(Some(item));
    };

    let save_item = move |_| {
        let editing = editing_item.get();
        let creating = creating_item.get();
        if let Ok(price_val) = price.get().parse::<f64>() {
            if let Ok(cat_id) = category_id.get().parse::<Uuid>() {
                let ts = track_stock.get();
                let sq = if ts { stock_quantity.get().parse::<i32>().ok() } else { None };
                let ki = Some(kitchen_item.get());

                if creating {
                    let n = name.get();
                    let d = Some(description.get()).filter(|s| !s.is_empty());
                    let s = Some(sku.get()).filter(|s| !s.is_empty());
                    let stock = Some(in_stock.get());
                    let img_data = image_preview.get();
                    leptos::task::spawn_local(async move {
                        if let Ok(new_item) = create_item(n, d, price_val, cat_id, s, stock, sq, ki).await {
                            if let Some(data) = img_data {
                                if data.starts_with("data:") {
                                    let _ = upload_item_image(new_item.id, data).await;
                                }
                            }
                            set_creating_item.set(false);
                            set_image_preview.set(None);
                            set_reload.update(|v| *v += 1);
                        }
                    });
                } else if let Some(item) = editing {
                    let n = Some(name.get());
                    let d = Some(description.get()).filter(|s| !s.is_empty());
                    let s = Some(sku.get()).filter(|s| !s.is_empty());
                    let stock = Some(in_stock.get());
                    let item_id = item.id;
                    let img_data = image_preview.get();
                    let had_image = item.image_path.is_some();
                    leptos::task::spawn_local(async move {
                        if update_item(item_id, n, d, Some(price_val), Some(cat_id), s, stock, sq, Some(ts), ki).await.is_ok() {
                            match img_data.as_deref() {
                                Some(data) if data.starts_with("data:") => {
                                    let _ = upload_item_image(item_id, data.to_string()).await;
                                }
                                None if had_image => {
                                    let _ = remove_item_image(item_id).await;
                                }
                                _ => {}
                            }
                            set_editing_item.set(None);
                            set_image_preview.set(None);
                            set_reload.update(|v| *v += 1);
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
                if delete_item(id).await.is_ok() { set_deleting_item.set(None); set_reload.update(|v| *v += 1); }
            });
        }
    };
    let cancel_delete = move |_| { set_deleting_item.set(None); };
    let cancel_edit = move |_| {
        set_editing_item.set(None); set_creating_item.set(false);
        set_name.set(String::new()); set_description.set(String::new());
        set_price.set(String::new()); set_category_id.set(String::new());
        set_sku.set(String::new()); set_in_stock.set(true);
        set_image_preview.set(None); set_track_stock.set(false);
        set_stock_quantity.set(String::new()); set_kitchen_item.set(false);
    };
    let start_create = move |_| {
        set_name.set(String::new()); set_description.set(String::new());
        set_price.set(String::new());
        set_category_id.set(if let Some(cat) = categories.get().first() { cat.id.to_string() } else { String::new() });
        set_sku.set(String::new()); set_in_stock.set(true);
        set_image_preview.set(None); set_track_stock.set(false);
        set_stock_quantity.set(String::new()); set_kitchen_item.set(false);
        set_creating_item.set(true); set_editing_item.set(None);
    };

    let on_image_selected = move |ev: leptos::ev::Event| {
        handle_image_file(ev, set_image_preview);
    };
    let remove_image = move |_| { set_image_preview.set(None); };

    view! {
        <div>
            <div class="page-header">
                <h2>"Items"</h2>
                <button class="btn-primary" on:click=start_create
                    disabled=move || editing_item.get().is_some() || creating_item.get()
                >"Add New Item"</button>
            </div>

            <Show when=move || deleting_item.get().is_some() fallback=|| ()>
                {move || {
                    deleting_item.get().map(|(_, name)| view! {
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
                        <div class="form-group">
                            <label>
                                <input type="checkbox" checked=move || kitchen_item.get() on:change=move |ev| set_kitchen_item.set(event_target_checked(&ev)) />
                                " Kitchen Item"
                            </label>
                        </div>
                        <div class="form-group">
                            <label>
                                <input type="checkbox" checked=move || track_stock.get() on:change=move |ev| set_track_stock.set(event_target_checked(&ev)) />
                                " Track Stock Quantity"
                            </label>
                            <Show when=move || track_stock.get() fallback=|| view! { <span class="text-muted">"Endless (untracked)"</span> }>
                                <input type="number" min="0" placeholder="Quantity"
                                    value=move || stock_quantity.get()
                                    on:input=move |ev| set_stock_quantity.set(event_target_value(&ev))
                                />
                            </Show>
                        </div>
                        <div class="form-group">
                            <label>"Image"</label>
                            <input type="file" accept="image/*" on:change=on_image_selected />
                            <Show when=move || image_preview.get().is_some() fallback=|| ()>
                                <div class="image-preview-container">
                                    <img class="image-preview" src=move || image_preview.get().unwrap_or_default() />
                                    <button type="button" class="btn-small btn-danger" on:click=remove_image>"Remove"</button>
                                </div>
                            </Show>
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn-success" on:click=save_item>"Save"</button>
                        <button class="btn-secondary" on:click=cancel_edit>"Cancel"</button>
                    </div>
                </div>
            </Show>

            <table class="data-table">
                <thead><tr><th>"Image"</th><th>"Name"</th><th>"Price"</th><th>"Category"</th><th>"Stock"</th><th>"Kitchen"</th><th></th></tr></thead>
                <tbody>
                    <For each=move || items.get() key=|i| (i.id, i.name.clone(), i.price.to_bits(), i.in_stock, i.sku.clone(), i.category_id, i.image_path.clone(), i.stock_quantity, i.kitchen_item) let:item>
                        {
                            let item_clone = item.clone();
                            let item_id = item.id;
                            let item_name = item.name.clone();
                            let category_name = categories.get().iter()
                                .find(|c| c.id == item.category_id)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| "Unknown".to_string());
                            let stock_display = match item.stock_quantity {
                                Some(q) => format!("{}", q),
                                None => if item.in_stock { "Endless".to_string() } else { "Out".to_string() },
                            };
                            view! {
                                <tr>
                                    <td class="item-thumb-cell">
                                        {item.image_path.clone().map(|path| view! { <img class="item-thumb" src=path alt="" /> })}
                                    </td>
                                    <td>{item.name.clone()}</td>
                                    <td>{format!("{} {:.2}", CURRENCY_SYMBOL, item.price)}</td>
                                    <td>{category_name}</td>
                                    <td>{stock_display}</td>
                                    <td>{if item.kitchen_item { "Yes" } else { "-" }}</td>
                                    <td class="data-table-actions">
                                        <button class="btn-small" on:click=move |_| start_edit(item_clone.clone())
                                            disabled=move || editing_item.get().is_some() || creating_item.get()
                                        >"Edit"</button>
                                        <button class="btn-small btn-danger" on:click=move |_| confirm_delete(item_id, item_name.clone())
                                            disabled=move || editing_item.get().is_some() || creating_item.get()
                                        >"Delete"</button>
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
