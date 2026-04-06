use chrono::{DateTime, Utc};
use leptos::prelude::*;
use uuid::Uuid;

use crate::models::*;
use crate::server_fns::*;

const CURRENCY_SYMBOL: &str = "€";

fn format_elapsed(created_at: DateTime<Utc>, _tick: u32) -> String {
    let elapsed = Utc::now().signed_duration_since(created_at);
    let total_secs = elapsed.num_seconds().max(0);
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{}:{:02}", mins, secs)
}

#[cfg(target_arch = "wasm32")]
fn setup_tick(set_tick: WriteSignal<u32>) {
    use wasm_bindgen::prelude::*;
    let cb = Closure::wrap(Box::new(move || {
        set_tick.update(|v| *v = v.wrapping_add(1));
    }) as Box<dyn Fn()>);
    let _ = web_sys::window().unwrap()
        .set_interval_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(), 1000,
        );
    cb.forget();
}

#[cfg(target_arch = "wasm32")]
fn setup_kitchen_ws(set_reload: WriteSignal<u32>) {
    use wasm_bindgen::prelude::*;

    fn connect(set_reload: WriteSignal<u32>) {
        let win = web_sys::window().unwrap();
        let loc = win.location();
        let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss:" } else { "ws:" };
        let host = loc.host().unwrap_or_default();
        let url = format!("{}//{}/ws/kitchen", proto, host);
        let Ok(ws) = web_sys::WebSocket::new(&url) else { return };

        let sr = set_reload;
        let onmessage = Closure::wrap(Box::new(move |_: web_sys::MessageEvent| {
            sr.update(|v| *v += 1);
        }) as Box<dyn Fn(_)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onclose = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
            let sr2 = set_reload;
            let cb = Closure::wrap(Box::new(move || { connect(sr2); }) as Box<dyn Fn()>);
            let _ = web_sys::window().unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 2000);
            cb.forget();
        }) as Box<dyn Fn(_)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();
    }
    connect(set_reload);
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_kitchen_ws(_set_reload: WriteSignal<u32>) {}

#[cfg(not(target_arch = "wasm32"))]
fn setup_tick(_set_tick: WriteSignal<u32>) {}

#[component]
pub fn SalePage() -> impl IntoView {
    let (categories, set_categories) = signal(Vec::<Category>::new());
    let (items, set_items) = signal(Vec::<Item>::new());
    let (selected_category, set_selected_category) = signal(Option::<Uuid>::None);
    let (current_transaction, set_current_transaction) = signal(Option::<Uuid>::None);
    let (transaction_items, set_transaction_items) =
        signal(Vec::<TransactionItemDetail>::new());
    let (customer_name, set_customer_name) = signal(String::new());
    let (change_amount, set_change_amount) = signal(Option::<f64>::None);
    let (open_transactions, set_open_transactions) = signal(Vec::<Transaction>::new());
    let (show_open_transactions, set_show_open_transactions) = signal(false);
    let (payment_amount, set_payment_amount) = signal(String::new());
    let (canceling_transaction, set_canceling_transaction) = signal(Option::<Uuid>::None);
    let (last_closed_transaction, set_last_closed_transaction) =
        signal(Option::<Transaction>::None);
    let (use_quick_cash, set_use_quick_cash) = signal(true);
    let (active_tab, set_active_tab) = signal("sale".to_string());
    let (kitchen_orders, set_kitchen_orders) = signal(Vec::<KitchenOrder>::new());
    let (reload_items, set_reload_items) = signal(0u32);
    let (tick, set_tick) = signal(0u32);

    Effect::new(move || { setup_tick(set_tick); });

    let fetch_last_closed = move || {
        leptos::task::spawn_local(async move {
            if let Ok(all_transactions) = fetch_all_transactions().await {
                let last_closed = all_transactions
                    .iter()
                    .filter(|t| t.status == "closed" && t.change_amount.is_some())
                    .max_by_key(|t| t.closed_at);
                set_last_closed_transaction.set(last_closed.cloned());
            }
        });
    };

    Effect::new(move || {
        reload_items.get();
        leptos::task::spawn_local(async move {
            if let Ok(cats) = fetch_categories().await {
                set_categories.set(cats);
            }
            if let Ok(its) = fetch_items().await {
                set_items.set(its);
            }
            if let Ok(trans) = fetch_open_transactions().await {
                set_open_transactions.set(trans);
            }
        });
    });

    let (reload_kitchen, set_reload_kitchen) = signal(0u32);
    Effect::new(move || { setup_kitchen_ws(set_reload_kitchen); });
    Effect::new(move || {
        reload_kitchen.get();
        leptos::task::spawn_local(async move {
            // Fetch both pending and recently completed orders
            let mut all_orders = Vec::new();
            if let Ok(pending) = fetch_kitchen_orders().await {
                all_orders.extend(pending);
            }
            if let Ok(completed) = fetch_completed_kitchen_orders().await {
                // Only include completed orders not already in pending
                let pending_ids: Vec<Uuid> = all_orders.iter().map(|o| o.transaction_id).collect();
                for order in completed {
                    if !pending_ids.contains(&order.transaction_id) {
                        all_orders.push(order);
                    }
                }
            }
            set_kitchen_orders.set(all_orders);
        });
    });

    let filtered_items = move || {
        let all_items = items.get();
        match selected_category.get() {
            Some(cat_id) => all_items
                .into_iter()
                .filter(|item| item.category_id == cat_id)
                .collect(),
            None => all_items,
        }
    };

    let transaction_total = move || {
        transaction_items.get().iter().map(|i| i.total_price).sum::<f64>()
    };

    let start_transaction = move |_| {
        let name = customer_name.get();
        leptos::task::spawn_local(async move {
            let cust = if name.is_empty() { None } else { Some(name) };
            if let Ok(transaction) = create_transaction(cust).await {
                set_current_transaction.set(Some(transaction.id));
                set_transaction_items.set(vec![]);
                set_change_amount.set(None);
                if let Ok(trans) = fetch_open_transactions().await {
                    set_open_transactions.set(trans);
                }
            }
        });
    };

    let resume_transaction = move |trans_id: Uuid| {
        leptos::task::spawn_local(async move {
            if let Ok(details) = fetch_transaction_details(trans_id).await {
                set_current_transaction.set(Some(trans_id));
                set_transaction_items.set(details.items);
                set_customer_name.set(details.transaction.customer_name.unwrap_or_default());
                set_show_open_transactions.set(false);
            }
        });
    };

    let do_update_transaction = move |_| {
        let current_trans = current_transaction.get();
        let name = customer_name.get();
        let cust = if name.is_empty() { None } else { Some(name) };
        if let Some(trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                let _ = update_transaction_details(trans_id, cust).await;
            });
        }
    };

    let add_item = move |item: Item| {
        let current_trans = current_transaction.get();
        if let Some(trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                if add_item_to_transaction(trans_id, item.id, 1).await.is_ok() {
                    if let Ok(details) = fetch_transaction_details(trans_id).await {
                        set_transaction_items.set(details.items);
                    }
                }
            });
        }
    };

    let remove_item = move |item_id: Uuid| {
        let current_trans = current_transaction.get();
        if let Some(trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                if remove_item_from_transaction(trans_id, item_id).await.is_ok() {
                    if let Ok(details) = fetch_transaction_details(trans_id).await {
                        set_transaction_items.set(details.items);
                    }
                }
            });
        }
    };

    let checkout = move |_| {
        let current_trans = current_transaction.get();
        let amount_str = payment_amount.get();
        let fetch_last_closed = fetch_last_closed.clone();
        if let Some(trans_id) = current_trans {
            if let Ok(amount) = amount_str.parse::<f64>() {
                leptos::task::spawn_local(async move {
                    if let Ok(response) = close_transaction(trans_id, amount).await {
                        set_change_amount.set(Some(response.change_amount));
                        set_current_transaction.set(None);
                        set_customer_name.set(String::new());
                        set_payment_amount.set(String::new());
                        if let Ok(trans) = fetch_open_transactions().await {
                            set_open_transactions.set(trans);
                        }
                        fetch_last_closed();
                        set_reload_items.update(|v| *v += 1);
                        set_reload_kitchen.update(|v| *v += 1);
                    }
                });
            }
        }
    };

    let confirm_cancel_sale = move |id: Uuid| {
        set_canceling_transaction.set(Some(id));
    };

    let cancel_sale_handler = move |_| {
        let current_trans = current_transaction.get();
        let fetch_last_closed = fetch_last_closed.clone();
        if let Some(trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                if cancel_transaction(trans_id).await.is_ok() {
                    set_current_transaction.set(None);
                    set_transaction_items.set(vec![]);
                    set_customer_name.set(String::new());
                    if let Ok(trans) = fetch_open_transactions().await {
                        set_open_transactions.set(trans);
                    }
                    fetch_last_closed();
                }
            });
        }
        set_canceling_transaction.set(None);
    };

    let cancel_cancel_sale = move |_| {
        set_canceling_transaction.set(None);
    };

    let pause_sale = move |_| {
        let current_trans = current_transaction.get();
        if let Some(_trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                set_current_transaction.set(None);
                set_transaction_items.set(vec![]);
                set_customer_name.set(String::new());
                if let Ok(trans) = fetch_open_transactions().await {
                    set_open_transactions.set(trans);
                }
            });
        }
    };

    view! {
        <Show when=move || canceling_transaction.get().is_some() fallback=|| ()>
            {move || {
                canceling_transaction.get().map(|_| {
                    view! {
                        <div class="modal-overlay">
                            <div class="confirmation-modal">
                                <h3>"Confirm Delete"</h3>
                                <p>"Are you sure you want to delete this transaction?"</p>
                                <p class="warning-text">"This action cannot be undone."</p>
                                <div class="modal-actions">
                                    <button class="btn-danger" on:click=cancel_sale_handler>"Delete"</button>
                                    <button class="btn-secondary" on:click=cancel_cancel_sale>"Cancel"</button>
                                </div>
                            </div>
                        </div>
                    }
                })
            }}
        </Show>

        <div class="sale-page">
            // Sale/Kitchen tab switcher
            <div class="sale-tab-bar">
                <button
                    class=move || if active_tab.get() == "sale" { "sale-tab active" } else { "sale-tab" }
                    on:click=move |_| set_active_tab.set("sale".to_string())
                >"Sale"</button>
                <button
                    class=move || if active_tab.get() == "kitchen" { "sale-tab active" } else { "sale-tab" }
                    on:click=move |_| { set_active_tab.set("kitchen".to_string()); set_reload_kitchen.update(|v| *v += 1); }
                >"Kitchen"</button>
            </div>

            <Show when=move || active_tab.get() == "sale" fallback=move || view! {
                // Kitchen status tab (read-only)
                <div class="kitchen-status-panel">
                    <h3>"Kitchen Orders"</h3>
                    <Show when=move || kitchen_orders.get().is_empty() fallback=move || view! {
                        <For each=move || kitchen_orders.get()
                            key=|o| (o.transaction_id, o.items.iter().filter(|i| i.completed).count())
                            let:order
                        >
                            {
                                let created = order.created_at;
                                let all_done = order.items.iter().all(|i| i.completed);
                                let card_class = if all_done { "kitchen-status-card kitchen-status-card-done" } else { "kitchen-status-card" };
                                view! {
                            <div class=card_class>
                                <div class="kitchen-status-header">
                                    <strong>{order.customer_name.clone().unwrap_or_else(|| "Walk-in".to_string())}</strong>
                                    {if all_done {
                                        view! { <span class="kitchen-status-time kitchen-status-complete-badge">"Complete"</span> }.into_any()
                                    } else {
                                        view! { <span class="kitchen-status-time">{move || format_elapsed(created, tick.get())}</span> }.into_any()
                                    }}
                                </div>
                                <ul class="kitchen-status-items">
                                    <For each=move || order.items.clone() key=|i| (i.transaction_item_id, i.completed) let:item>
                                        <li class=if item.completed { "kitchen-status-item-done" } else { "" }>
                                            {if item.completed { "✓ " } else { "" }}
                                            {format!("{}x {}", item.quantity, item.item_name)}
                                        </li>
                                    </For>
                                </ul>
                            </div>
                                }
                            }
                        </For>
                    }>
                        <p class="kitchen-empty">"No pending kitchen orders"</p>
                    </Show>
                </div>
            }>

            <div class="sale-grid">
                <div class="items-section">
                    <h2>"Items"</h2>
                    <div class="category-tabs">
                        <button
                            class=move || if selected_category.get().is_none() { "active" } else { "" }
                            on:click=move |_| set_selected_category.set(None)
                        >"All"</button>
                        <For each=move || categories.get() key=|cat| cat.id let:cat>
                            {
                                let cat_id = cat.id;
                                view! {
                                    <button
                                        class=move || if selected_category.get() == Some(cat_id) { "active" } else { "" }
                                        on:click=move |_| set_selected_category.set(Some(cat_id))
                                    >{cat.name.clone()}</button>
                                }
                            }
                        </For>
                    </div>
                    <div class="items-grid">
                        <For each=filtered_items key=|item| (item.id, item.name.clone(), item.price.to_bits(), item.in_stock, item.image_path.clone(), item.stock_quantity) let:item>
                            {
                                let item_clone = item.clone();
                                let has_image = item.image_path.is_some();
                                let card_class = if has_image { "item-card item-card-has-image" } else { "item-card" };
                                let stock_warn = item.stock_quantity.filter(|&q| q > 0 && q <= 5);
                                let is_out = !item.in_stock || item.stock_quantity.map(|q| q <= 0).unwrap_or(false);
                                view! {
                                    <button
                                        class=card_class
                                        on:click=move |_| add_item(item_clone.clone())
                                        disabled=move || current_transaction.get().is_none() || is_out
                                    >
                                        {item.image_path.clone().map(|path| view! {
                                            <img class="item-card-img" src=path alt="" />
                                        })}
                                        <div class="item-card-overlay">
                                            <div class="item-price-badge">{format!("{}{:.2}", CURRENCY_SYMBOL, item.price)}</div>
                                            <div class="item-name-badge">{item.name.clone()}</div>
                                        </div>
                                        <Show when=move || is_out fallback=|| ()>
                                            <div class="out-of-stock">"Out of Stock"</div>
                                        </Show>
                                        {stock_warn.map(|q| view! {
                                            <div class="stock-warning">{format!("{} left", q)}</div>
                                        })}
                                    </button>
                                }
                            }
                        </For>
                    </div>
                </div>

                <div class="transaction-section">
                    <Show
                        when=move || current_transaction.get().is_some()
                        fallback=move || view! {
                            <div class="start-transaction">
                                <input
                                    type="text"
                                    placeholder="Customer name (optional)"
                                    on:input=move |ev| set_customer_name.set(event_target_value(&ev))
                                    value=move || customer_name.get()
                                />
                                <button class="btn-primary" on:click=start_transaction>"New Transaction"</button>

                                <Show when=move || !open_transactions.get().is_empty() fallback=|| ()>
                                    <button
                                        class="btn-secondary"
                                        on:click=move |_| set_show_open_transactions.set(!show_open_transactions.get())
                                    >
                                        {move || if show_open_transactions.get() { "Hide" } else { "Show" }}
                                        " Open Transactions ("{move || open_transactions.get().len()}")"
                                    </button>
                                </Show>

                                <Show when=move || last_closed_transaction.get().is_some() fallback=|| ()>
                                {
                                    last_closed_transaction.get().map(|t| {
                                        view! {
                                            <div class="last-change-display">
                                                <strong>"Last Change: "</strong>
                                                {format!("{} {:.2}", CURRENCY_SYMBOL, t.change_amount.unwrap())}
                                            </div>
                                        }
                                    })
                                }
                                </Show>

                                <Show when=move || show_open_transactions.get() fallback=|| ()>
                                    <div class="open-transactions-list">
                                        <For each=move || open_transactions.get() key=|t| t.id let:trans>
                                            {
                                                let trans_id = trans.id;
                                                view! {
                                                    <div class="open-transaction-item">
                                                        <div>
                                                            <strong>{trans.customer_name.clone().unwrap_or_else(|| "Walk-in".to_string())}</strong>
                                                            <span>" - "{format!("{} {:.2}", CURRENCY_SYMBOL, trans.total)}</span>
                                                        </div>
                                                        <button class="btn-small" on:click=move |_| resume_transaction(trans_id)>"Resume"</button>
                                                    </div>
                                                }
                                            }
                                        </For>
                                    </div>
                                </Show>
                            </div>
                        }
                    >
                        <div class="transaction-active">
                            <div class="transaction-header">
                                <table class="customer-table"><tbody>
                                    <tr>
                                        <td><strong>"Customer: "</strong></td>
                                        <td>
                                            <input type="text" placeholder="Walk-in"
                                                on:input=move |ev| set_customer_name.set(event_target_value(&ev))
                                                value=move || customer_name.get()
                                            />
                                        </td>
                                        <td class="customer-table-actions">
                                            <button class="btn-primary-small" on:click=do_update_transaction>"Update"</button>
                                        </td>
                                    </tr>
                                </tbody></table>
                            </div>

                            <div class="transaction-items">
                                <table class="data-table"><tbody>
                                    <For each=move || transaction_items.get() key=|item| (item.id, item.quantity) let:item>
                                        {
                                            let item_id = item.item_id;
                                            view! {
                                                <tr>
                                                    <td>{item.item_name.clone()}</td>
                                                    <td>{format!("{}x", item.quantity)}</td>
                                                    <td>{format!("{} {:.2}", CURRENCY_SYMBOL, item.total_price)}</td>
                                                    <td class="data-table-actions">
                                                        <button class="btn-remove" on:click=move |_| remove_item(item_id)>"-"</button>
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    </For>
                                </tbody></table>
                            </div>

                            <div class="transaction-total">
                                <strong>"Total: "</strong>
                                <strong>{move || format!("{} {:.2}", CURRENCY_SYMBOL, transaction_total())}</strong>
                            </div>

                            <div class="payment-change-wrapper">
                                <div class="payment-section">
                                    <strong>"Cash: "</strong>
                                    <input type="text" class="payment-input" placeholder="" readonly value=move || payment_amount.get() />
                                </div>
                                <div class="change-section">
                                    <strong>"Change: "</strong>
                                    <input type="text" class="change-input" placeholder="" readonly
                                        value=move || {
                                            match payment_amount.get().parse::<f64>() {
                                                Ok(amount) => format!("{:.2}", amount - transaction_total()),
                                                Err(_) => String::new(),
                                            }
                                        }
                                    />
                                </div>
                            </div>

                            <div class="keypad-section">
                                // Toggle between number keys and quick cash
                                <div class="keypad-mode-toggle">
                                    <button
                                        class=move || if !use_quick_cash.get() { "keypad-toggle-btn active" } else { "keypad-toggle-btn" }
                                        on:click=move |_| set_use_quick_cash.set(false)
                                    >"Keypad"</button>
                                    <button
                                        class=move || if use_quick_cash.get() { "keypad-toggle-btn active" } else { "keypad-toggle-btn" }
                                        on:click=move |_| set_use_quick_cash.set(true)
                                    >"Quick Cash"</button>
                                </div>

                                <Show when=move || !use_quick_cash.get() fallback=move || view! {
                                    // Quick cash note buttons
                                    <div class="quick-cash-grid">
                                        <For each=|| vec![5, 10, 20, 50, 100, 200] key=|v| *v let:val>
                                            {
                                                let v = val;
                                                view! {
                                                    <button class="quick-cash-btn"
                                                        on:click=move |_| set_payment_amount.set(format!("{}", v))
                                                    >{format!("{}{}", CURRENCY_SYMBOL, v)}</button>
                                                }
                                            }
                                        </For>
                                        <button class="quick-cash-btn quick-cash-exact"
                                            on:click=move |_| set_payment_amount.set(format!("{:.2}", transaction_total()))
                                        >"Exact"</button>
                                        <button class="quick-cash-btn quick-cash-clear"
                                            on:click=move |_| set_payment_amount.set(String::new())
                                        >"Clear"</button>
                                    </div>
                                }>
                                    <div class="keypad">
                                        <For each=|| vec!["7","8","9","4","5","6","1","2","3","0",".","⌫"] key=|val| val.to_string() let:val>
                                            {
                                                let val_clone = val.to_string();
                                                view! {
                                                    <button class="keypad-btn" on:click=move |_| {
                                                        if val_clone == "⌫" {
                                                            set_payment_amount.update(|amt| { amt.pop(); });
                                                        } else {
                                                            set_payment_amount.update(|amt| amt.push_str(&val_clone));
                                                        }
                                                    }>{val}</button>
                                                }
                                            }
                                        </For>
                                    </div>
                                </Show>
                            </div>

                            <div class="action-buttons">
                                <button class="action-button cancel" on:click=move |_| confirm_cancel_sale(current_transaction.get().unwrap_or_default())>"Cancel"</button>
                                <button class="action-button pause" on:click=pause_sale>"Back"</button>
                                <button class="action-button sale" on:click=checkout>"Checkout"</button>
                            </div>

                            <Show when=move || change_amount.get().is_some() fallback=|| ()>
                                <div class="change-display">
                                    <h3>"Change: "{move || format!("{} {:.2}", CURRENCY_SYMBOL, change_amount.get().unwrap())}</h3>
                                </div>
                            </Show>
                        </div>
                    </Show>
                </div>
            </div>
            </Show>
        </div>
    }
}
