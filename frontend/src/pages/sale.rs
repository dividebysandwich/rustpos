use leptos::prelude::*;
use uuid::Uuid;

use crate::models::*;
use crate::server_fns::*;

const CURRENCY_SYMBOL: &str = "€";

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
                        if let Ok(trans) = fetch_open_transactions().await {
                            set_open_transactions.set(trans);
                        }
                        fetch_last_closed();
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
                        <For each=filtered_items key=|item| item.id let:item>
                            {
                                let item_clone = item.clone();
                                let has_image = item.image_path.is_some();
                                let card_class = if has_image { "item-card item-card-has-image" } else { "item-card" };
                                view! {
                                    <button
                                        class=card_class
                                        on:click=move |_| add_item(item_clone.clone())
                                        disabled=move || current_transaction.get().is_none()
                                    >
                                        {item.image_path.clone().map(|path| view! {
                                            <img class="item-card-img" src=path alt="" />
                                        })}
                                        <div class="item-card-overlay">
                                            <div class="item-price-badge">{format!("{}{:.2}", CURRENCY_SYMBOL, item.price)}</div>
                                            <div class="item-name-badge">{item.name.clone()}</div>
                                        </div>
                                        <Show when=move || !item.in_stock fallback=|| ()>
                                            <div class="out-of-stock">"Out of Stock"</div>
                                        </Show>
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
                                <table class="customer-table">
                                    <tbody>
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
                                    </tbody>
                                </table>
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
        </div>
    }
}
