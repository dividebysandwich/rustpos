use leptos::prelude::*;
use uuid::Uuid;

use crate::models::TransactionItemDetail;
use crate::server_fns::fetch_transaction_details;

#[cfg(target_arch = "wasm32")]
fn setup_display_ws(set_msg: WriteSignal<String>) {
    use wasm_bindgen::prelude::*;

    fn connect(set_msg: WriteSignal<String>) {
        let win = web_sys::window().unwrap();
        let loc = win.location();
        let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss:" } else { "ws:" };
        let host = loc.host().unwrap_or_default();
        let url = format!("{}//{}/ws/display", proto, host);

        let Ok(ws) = web_sys::WebSocket::new(&url) else { return };

        let sm = set_msg;
        let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            if let Some(msg) = e.data().as_string() {
                sm.set(msg);
            }
        }) as Box<dyn Fn(_)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onclose = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
            let sm2 = set_msg;
            let cb = Closure::wrap(Box::new(move || { connect(sm2); }) as Box<dyn Fn()>);
            let _ = web_sys::window().unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(), 2000,
                );
            cb.forget();
        }) as Box<dyn Fn(_)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();
    }

    connect(set_msg);
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_display_ws(_set_msg: WriteSignal<String>) {}

#[cfg(target_arch = "wasm32")]
fn clear_timeout(id: i32) {
    web_sys::window().unwrap().clear_timeout_with_handle(id);
}

#[cfg(target_arch = "wasm32")]
fn set_timeout_ms(cb: impl Fn() + 'static, ms: i32) -> i32 {
    use wasm_bindgen::prelude::*;
    let cb = Closure::wrap(Box::new(cb) as Box<dyn Fn()>);
    let id = web_sys::window().unwrap()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(), ms,
        )
        .unwrap_or(0);
    cb.forget();
    id
}

#[cfg(target_arch = "wasm32")]
fn scroll_display_to_bottom() {
    use wasm_bindgen::prelude::*;
    let cb = Closure::wrap(Box::new(move || {
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.query_selector(".display-items").ok())
            .flatten()
        {
            el.set_scroll_top(el.scroll_height());
        }
    }) as Box<dyn Fn()>);
    let _ = web_sys::window().unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
    cb.forget();
}

#[component]
pub fn DisplayPage() -> impl IntoView {
    let currency = expect_context::<RwSignal<String>>();
    let (items, set_items) = signal(Vec::<TransactionItemDetail>::new());
    let (total, set_total) = signal(0.0f64);
    let (active, set_active) = signal(false);
    let (ws_msg, set_ws_msg) = signal(String::new());
    #[allow(unused_variables)]
    let (timer_id, set_timer_id) = signal(Option::<i32>::None);

    Effect::new(move || { setup_display_ws(set_ws_msg); });

    // Handle WebSocket messages
    Effect::new(move || {
        let msg = ws_msg.get();
        if msg.is_empty() { return; }

        if let Some(id_str) = msg.strip_prefix("update:") {
            if let Ok(id) = Uuid::parse_str(id_str) {
                // Cancel any pending clear timer
                #[cfg(target_arch = "wasm32")]
                if let Some(tid) = timer_id.get_untracked() {
                    clear_timeout(tid);
                }
                set_timer_id.set(None);

                leptos::task::spawn_local(async move {
                    if let Ok(details) = fetch_transaction_details(id).await {
                        set_items.set(details.items);
                        set_total.set(details.transaction.total);
                        set_active.set(true);
                    }
                });
            }
        } else if let Some(id_str) = msg.strip_prefix("closed:") {
            if let Ok(id) = Uuid::parse_str(id_str) {
                // Fetch final state
                leptos::task::spawn_local(async move {
                    if let Ok(details) = fetch_transaction_details(id).await {
                        set_items.set(details.items);
                        set_total.set(details.transaction.total);
                    }
                });

                // Clear display after 60 seconds
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(tid) = timer_id.get_untracked() {
                        clear_timeout(tid);
                    }
                    let tid = set_timeout_ms(move || {
                        set_active.set(false);
                        set_items.set(vec![]);
                        set_total.set(0.0);
                    }, 60_000);
                    set_timer_id.set(Some(tid));
                }
            }
        } else if msg == "clear" {
            #[cfg(target_arch = "wasm32")]
            if let Some(tid) = timer_id.get_untracked() {
                clear_timeout(tid);
            }
            set_timer_id.set(None);
            set_active.set(false);
            set_items.set(vec![]);
            set_total.set(0.0);
        }
    });

    // Auto-scroll items list to bottom on changes
    Effect::new(move || {
        items.get();
        #[cfg(target_arch = "wasm32")]
        scroll_display_to_bottom();
    });

    view! {
        <div class="display-page">
            <Show when=move || active.get() fallback=|| ()>
                <div class="display-items">
                    <For each=move || items.get() key=|item| (item.id, item.quantity) let:item>
                        <div class="display-item-row">
                            <span class="display-item-name">{item.item_name.clone()}</span>
                            <span class="display-item-qty">{format!("{}x", item.quantity)}</span>
                            <span class="display-item-price">{move || format!("{}{:.2}", currency.get(), item.total_price)}</span>
                        </div>
                    </For>
                </div>
                <div class="display-total">
                    <span>"Total"</span>
                    <span>{move || format!("{}{:.2}", currency.get(), total.get())}</span>
                </div>
            </Show>
        </div>
    }
}
