use chrono::{DateTime, Utc};
use leptos::prelude::*;
use uuid::Uuid;

use crate::models::*;
use crate::server_fns::*;

#[cfg(target_arch = "wasm32")]
fn setup_kitchen_ws(set_reload: WriteSignal<u32>) {
    use wasm_bindgen::prelude::*;

    fn connect(set_reload: WriteSignal<u32>) {
        let win = web_sys::window().unwrap();
        let loc = win.location();
        let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss:" } else { "ws:" };
        let host = loc.host().unwrap_or_default();
        let url = format!("{}//{}/ws/kitchen", proto, host);

        let ws = web_sys::WebSocket::new(&url).unwrap();

        let sr = set_reload;
        let onmessage = Closure::wrap(Box::new(move |_: web_sys::MessageEvent| {
            sr.update(|v| *v += 1);
        }) as Box<dyn Fn(_)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onclose = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
            let sr2 = set_reload;
            let cb = Closure::wrap(Box::new(move || {
                connect(sr2);
            }) as Box<dyn Fn()>);
            let _ = web_sys::window().unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(), 2000,
                );
            cb.forget();
        }) as Box<dyn Fn(_)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();
    }

    connect(set_reload);
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_kitchen_ws(_set_reload: WriteSignal<u32>) {}

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

#[cfg(not(target_arch = "wasm32"))]
fn setup_tick(_set_tick: WriteSignal<u32>) {}

fn format_elapsed(created_at: DateTime<Utc>, _tick: u32) -> String {
    let elapsed = Utc::now().signed_duration_since(created_at);
    let total_secs = elapsed.num_seconds().max(0);
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{}:{:02}", mins, secs)
}

fn redirect_to_login() {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = web_sys::window().unwrap().location().set_href("/login");
    }
}

#[component]
pub fn KitchenPage() -> impl IntoView {
    let (authorized, set_authorized) = signal(false);

    // Auth check
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match get_current_user().await {
                Ok(Some(u)) if u.role == "admin" || u.role == "cook" => {
                    set_authorized.set(true);
                }
                _ => redirect_to_login(),
            }
        });
    });

    let do_logout = move |_| {
        leptos::task::spawn_local(async move {
            let _ = logout().await;
            redirect_to_login();
        });
    };

    let (orders, set_orders) = signal(Vec::<KitchenOrder>::new());
    let (completed_orders, set_completed_orders) = signal(Vec::<KitchenOrder>::new());
    let (show_completed, set_show_completed) = signal(false);
    let (reload, set_reload) = signal(0u32);
    // Track known order IDs to detect new arrivals
    let (known_orders, set_known_orders) = signal(Vec::<Uuid>::new());
    // New orders that should blink (will be cleared after 3s)
    let (new_orders, set_new_orders) = signal(Vec::<Uuid>::new());
    // Tick signal for elapsed time display
    let (tick, set_tick) = signal(0u32);

    Effect::new(move || {
        setup_kitchen_ws(set_reload);
        setup_tick(set_tick);
    });

    Effect::new(move || {
        reload.get();
        leptos::task::spawn_local(async move {
            if let Ok(o) = fetch_kitchen_orders().await {
                // Detect new orders
                let prev = known_orders.get();
                let mut fresh: Vec<Uuid> = Vec::new();
                for order in &o {
                    if !prev.contains(&order.transaction_id) {
                        fresh.push(order.transaction_id);
                    }
                }
                if !fresh.is_empty() {
                    set_new_orders.update(|v| v.extend(fresh.iter().cloned()));
                    // Clear blink after 3 seconds
                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::prelude::*;
                        let fresh_clone = fresh.clone();
                        let cb = Closure::wrap(Box::new(move || {
                            set_new_orders.update(|v| v.retain(|id| !fresh_clone.contains(id)));
                        }) as Box<dyn Fn()>);
                        let _ = web_sys::window().unwrap()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(
                                cb.as_ref().unchecked_ref(), 3000,
                            );
                        cb.forget();
                    }
                }
                set_known_orders.set(o.iter().map(|o| o.transaction_id).collect());
                set_orders.set(o);
            }
            if show_completed.get() {
                if let Ok(c) = fetch_completed_kitchen_orders().await {
                    set_completed_orders.set(c);
                }
            }
        });
    });

    let mark_item_done = move |ti_id: Uuid| {
        // Optimistic update: mutate orders signal directly so the <For> key changes
        set_orders.update(|orders| {
            for order in orders.iter_mut() {
                for item in order.items.iter_mut() {
                    if item.transaction_item_id == ti_id {
                        item.completed = true;
                    }
                }
            }
        });
        leptos::task::spawn_local(async move {
            let _ = complete_kitchen_item(ti_id).await;
        });
    };

    let mark_order_done = move |t_id: Uuid| {
        set_orders.update(|orders| {
            for order in orders.iter_mut() {
                if order.transaction_id == t_id {
                    for item in order.items.iter_mut() {
                        item.completed = true;
                    }
                }
            }
        });
        leptos::task::spawn_local(async move {
            let _ = complete_kitchen_order(t_id).await;
        });
    };

    let toggle_completed = move |_| {
        let new_val = !show_completed.get();
        set_show_completed.set(new_val);
        if new_val {
            leptos::task::spawn_local(async move {
                if let Ok(c) = fetch_completed_kitchen_orders().await {
                    set_completed_orders.set(c);
                }
            });
        }
    };

    view! {
        <Show when=move || authorized.get() fallback=|| view! { <div class="loading">"Loading..."</div> }>
        <div class="kitchen-page">
            <div class="kitchen-header">
                <h1>"Kitchen Display"</h1>
                <div class="kitchen-header-actions">
                    <button
                        class=move || if show_completed.get() { "btn-primary kitchen-header-btn" } else { "btn-secondary kitchen-header-btn" }
                        on:click=toggle_completed
                    >{move || if show_completed.get() { "Hide Completed" } else { "Show Completed" }}</button>
                    <button class="btn-primary kitchen-header-btn" on:click=move |_| set_reload.update(|v| *v += 1)>"Refresh"</button>
                    <button class="btn-secondary kitchen-header-btn" on:click=do_logout>"Logout"</button>
                </div>
            </div>

            <Show when=move || !orders.get().is_empty() fallback=move || view! {
                <div class="kitchen-empty-msg">
                    <h2>"No pending orders"</h2>
                    <p>"New kitchen orders will appear here automatically"</p>
                </div>
            }>
                <div class="kitchen-grid">
                    <For each=move || orders.get() key=|o| (o.transaction_id, o.items.iter().filter(|i| i.completed).count()) let:order>
                        {
                            let t_id = order.transaction_id;
                            let created = order.created_at;
                            let all_done = order.items.iter().all(|i| i.completed);
                            let is_new = new_orders.get().contains(&t_id);
                            let card_class = match (all_done, is_new) {
                                (true, _) => "kitchen-order-card kitchen-order-all-done",
                                (false, true) => "kitchen-order-card kitchen-order-new",
                                (false, false) => "kitchen-order-card",
                            };
                            view! {
                                <div class=card_class>
                                    <div class="kitchen-order-header">
                                        <span class="kitchen-customer">
                                            {order.customer_name.clone().unwrap_or_else(|| "Walk-in".to_string())}
                                        </span>
                                        <span class="kitchen-time">{move || format_elapsed(created, tick.get())}</span>
                                    </div>
                                    <div class="kitchen-order-items">
                                        <For each=move || order.items.clone() key=|i| (i.transaction_item_id, i.completed) let:item>
                                            {
                                                let ti_id = item.transaction_item_id;
                                                let done = item.completed;
                                                view! {
                                                    <div class=if done { "kitchen-item-row kitchen-item-done" } else { "kitchen-item-row" }>
                                                        <span class="kitchen-item-qty">{format!("{}x", item.quantity)}</span>
                                                        <span class="kitchen-item-name">{item.item_name.clone()}</span>
                                                        {if done {
                                                            view! { <span class="kitchen-done-check">"Done"</span> }.into_any()
                                                        } else {
                                                            view! {
                                                                <button class="kitchen-done-btn"
                                                                    on:click=move |_| mark_item_done(ti_id)
                                                                >"Done"</button>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                }
                                            }
                                        </For>
                                    </div>
                                    <button class="kitchen-complete-order-btn"
                                        on:click=move |_| mark_order_done(t_id)
                                        disabled=all_done
                                    >{if all_done { "Order Complete" } else { "Complete Order" }}</button>
                                </div>
                            }
                        }
                    </For>
                </div>
            </Show>

            <Show when=move || show_completed.get() fallback=|| ()>
                <h2 class="kitchen-section-title">"Completed Orders"</h2>
                <Show when=move || !completed_orders.get().is_empty() fallback=|| view! {
                    <p class="kitchen-empty">"No completed orders yet"</p>
                }>
                    <div class="kitchen-grid">
                        <For each=move || completed_orders.get() key=|o| o.transaction_id let:order>
                            <div class="kitchen-order-card kitchen-order-completed">
                                <div class="kitchen-order-header">
                                    <span class="kitchen-customer">
                                        {order.customer_name.clone().unwrap_or_else(|| "Walk-in".to_string())}
                                    </span>
                                    <span class="kitchen-time">{order.created_at.format("%H:%M").to_string()}</span>
                                </div>
                                <div class="kitchen-order-items">
                                    <For each=move || order.items.clone() key=|i| i.transaction_item_id let:item>
                                        <div class="kitchen-item-row kitchen-item-done">
                                            <span class="kitchen-item-qty">{format!("{}x", item.quantity)}</span>
                                            <span class="kitchen-item-name">{item.item_name.clone()}</span>
                                            <span class="kitchen-done-check">"Done"</span>
                                        </div>
                                    </For>
                                </div>
                            </div>
                        </For>
                    </div>
                </Show>
            </Show>
        </div>
        </Show>
    }
}
