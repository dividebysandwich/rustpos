use leptos::prelude::*;

use crate::models::*;
use crate::server_fns::*;

const CURRENCY_SYMBOL: &str = "€";

#[component]
pub fn TransactionsPage() -> impl IntoView {
    let (transactions, set_transactions) = signal(Vec::<Transaction>::new());
    let (show_all, set_show_all) = signal(false);

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

    view! {
        <div>
            <div class="page-header">
                <h2>"Transactions"</h2>
                <button class="btn-secondary" on:click=move |_| set_show_all.set(!show_all.get())>
                    {move || if show_all.get() { "Show Open Only" } else { "Show All" }}
                </button>
            </div>

            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Customer"</th>
                        <th>"Total"</th>
                        <th>"Status"</th>
                        <th>"Created"</th>
                    </tr>
                </thead>
                <tbody>
                    <For each=move || transactions.get() key=|t| t.id let:transaction>
                        <tr class=move || match transaction.status.as_str() {
                            "open" => "status-open",
                            "closed" => "status-closed",
                            "cancelled" => "status-cancelled",
                            _ => ""
                        }>
                            <td>{transaction.customer_name.clone().unwrap_or_else(|| "Walk-in".to_string())}</td>
                            <td>{format!("{} {:.2}", CURRENCY_SYMBOL, transaction.total)}</td>
                            <td>{transaction.status.clone()}</td>
                            <td>{transaction.created_at.format("%Y-%m-%d %H:%M").to_string()}</td>
                        </tr>
                    </For>
                </tbody>
            </table>
        </div>
    }
}
