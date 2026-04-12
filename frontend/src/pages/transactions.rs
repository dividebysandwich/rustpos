use leptos::prelude::*;
use uuid::Uuid;

use crate::i18n::I18n;
use crate::models::*;
use crate::server_fns::*;

#[component]
pub fn TransactionsPage() -> impl IntoView {
    let i18n = expect_context::<RwSignal<I18n>>();
    let currency = expect_context::<RwSignal<String>>();
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

    let (transactions, set_transactions) = signal(Vec::<Transaction>::new());
    let (show_all, set_show_all) = signal(false);
    let (selected, set_selected) = signal(Option::<Uuid>::None);
    let (details, set_details) = signal(Option::<TransactionDetailsResponse>::None);

    Effect::new(move || {
        let show_all = show_all.get();
        leptos::task::spawn_local(async move {
            let trans = if show_all {
                fetch_all_transactions().await
            } else {
                fetch_open_transactions().await
            };
            if let Ok(trans) = trans {
                set_transactions.set(trans);
            }
        });
    });

    let on_row_click = move |id: Uuid| {
        if selected.get() == Some(id) {
            set_selected.set(None);
            set_details.set(None);
        } else {
            set_selected.set(Some(id));
            set_details.set(None);
            leptos::task::spawn_local(async move {
                if let Ok(d) = fetch_transaction_details(id).await {
                    set_details.set(Some(d));
                }
            });
        }
    };

    view! {
        <Show when=move || authorized.get() fallback=move || view! { <div class="loading">{move || i18n.get().t("general.loading")}</div> }>
        <div>
            <div class="page-header">
                <h2>{move || i18n.get().t("transactions.title")}</h2>
                <button class="btn-secondary" on:click=move |_| set_show_all.set(!show_all.get())>
                    {move || if show_all.get() { i18n.get().t("transactions.show_open") } else { i18n.get().t("transactions.show_all") }}
                </button>
            </div>

            <table class="data-table">
                <thead>
                    <tr>
                        <th>{move || i18n.get().t("transactions.customer")}</th>
                        <th>{move || i18n.get().t("transactions.total")}</th>
                        <th>{move || i18n.get().t("transactions.status")}</th>
                        <th>{move || i18n.get().t("transactions.created")}</th>
                    </tr>
                </thead>
                <tbody>
                    <For each=move || transactions.get() key=|t| t.id let:transaction>
                        {
                            let tid = transaction.id;
                            let is_selected = move || selected.get() == Some(tid);
                            view! {
                                <tr
                                    class=move || {
                                        let status = match transaction.status.as_str() {
                                            "open" => "status-open",
                                            "closed" => "status-closed",
                                            "cancelled" => "status-cancelled",
                                            _ => "",
                                        };
                                        if is_selected() {
                                            format!("{} row-selected", status)
                                        } else {
                                            status.to_string()
                                        }
                                    }
                                    on:click=move |_| on_row_click(tid)
                                    style="cursor: pointer;"
                                >
                                    <td>{transaction.customer_name.clone().unwrap_or_else(|| i18n.get().t("general.walkin"))}</td>
                                    <td>{format!("{} {:.2}", &currency.get(), transaction.total)}</td>
                                    <td>{transaction.status.clone()}</td>
                                    <td>{transaction.created_at.format("%Y-%m-%d %H:%M").to_string()}</td>
                                </tr>
                                <Show when=is_selected fallback=|| ()>
                                    <tr class="transaction-detail-row">
                                        <td colspan="4">
                                            <Show
                                                when=move || details.get().is_some()
                                                fallback=move || view! { <div class="loading">{move || i18n.get().t("general.loading")}</div> }
                                            >
                                                {move || details.get().map(|d| {
                                                    let t = d.transaction.clone();
                                                    let items = d.items.clone();
                                                    let has_customer = t.customer_name.is_some();
                                                    let customer = t.customer_name.clone().unwrap_or_default();
                                                    let total = format!("{} {:.2}", &currency.get(), t.total);
                                                    let has_paid = t.paid_amount.is_some();
                                                    let paid = format!("{} {:.2}", &currency.get(), t.paid_amount.unwrap_or(0.0));
                                                    let has_change = t.change_amount.is_some();
                                                    let change = format!("{} {:.2}", &currency.get(), t.change_amount.unwrap_or(0.0));
                                                    view! {
                                                        <div class="transaction-detail-panel">
                                                            <Show when=move || has_customer fallback=|| ()>
                                                                <div class="detail-field">
                                                                    <strong>{i18n.get().t("transactions.details_customer")}</strong>
                                                                    {customer.clone()}
                                                                </div>
                                                            </Show>

                                                            <table class="detail-items-table">
                                                                <thead>
                                                                    <tr>
                                                                        <th>{i18n.get().t("transactions.item")}</th>
                                                                        <th>{i18n.get().t("transactions.qty")}</th>
                                                                        <th>{i18n.get().t("transactions.unit_price")}</th>
                                                                        <th>{i18n.get().t("transactions.subtotal")}</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    <For
                                                                        each=move || items.clone()
                                                                        key=|i| i.id
                                                                        let:item
                                                                    >
                                                                        <tr>
                                                                            <td>{item.item_name.clone()}</td>
                                                                            <td>{item.quantity.to_string()}</td>
                                                                            <td>{format!("{} {:.2}", &currency.get(), item.unit_price)}</td>
                                                                            <td>{format!("{} {:.2}", &currency.get(), item.total_price)}</td>
                                                                        </tr>
                                                                    </For>
                                                                </tbody>
                                                            </table>

                                                            <div class="detail-summary">
                                                                <div class="detail-field">
                                                                    <strong>{i18n.get().t("transactions.total_label")}</strong>
                                                                    {total.clone()}
                                                                </div>
                                                                <Show when=move || has_paid fallback=|| ()>
                                                                    <div class="detail-field">
                                                                        <strong>{i18n.get().t("transactions.paid")}</strong>
                                                                        {paid.clone()}
                                                                    </div>
                                                                </Show>
                                                                <Show when=move || has_change fallback=|| ()>
                                                                    <div class="detail-field">
                                                                        <strong>{i18n.get().t("transactions.change")}</strong>
                                                                        {change.clone()}
                                                                    </div>
                                                                </Show>
                                                            </div>
                                                        </div>
                                                    }
                                                })}
                                            </Show>
                                        </td>
                                    </tr>
                                </Show>
                            }
                        }
                    </For>
                </tbody>
            </table>
        </div>
        </Show>
    }
}
