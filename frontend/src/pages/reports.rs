use chrono::{DateTime, Utc};
use leptos::prelude::*;

use crate::models::*;
use crate::server_fns::*;

const CURRENCY_SYMBOL: &str = "€";

#[component]
pub fn ReportsPage() -> impl IntoView {
    let (report, set_report) = signal(Option::<SalesReport>::None);
    let (report_type, set_report_type) = signal(String::from("daily"));
    let (start_date, set_start_date) = signal(String::new());
    let (end_date, set_end_date) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    Effect::new(move || {
        let today = Utc::now();
        let week_ago = today - chrono::Duration::days(7);
        set_end_date.set(today.format("%Y-%m-%d").to_string());
        set_start_date.set(week_ago.format("%Y-%m-%d").to_string());
    });

    let load_report = move |rtype: String| {
        set_loading.set(true);
        set_error.set(None);

        leptos::task::spawn_local(async move {
            let result: Result<SalesReport, String> = match rtype.as_str() {
                "daily" => fetch_daily_report().await.map_err(|e| e.to_string()),
                "monthly" => fetch_monthly_report().await.map_err(|e| e.to_string()),
                "custom" => {
                    if let (Ok(start), Ok(end)) = (
                        start_date.get().parse::<chrono::NaiveDate>(),
                        end_date.get().parse::<chrono::NaiveDate>(),
                    ) {
                        let start_dt = start.and_hms_opt(0, 0, 0)
                            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                        let end_dt = end.and_hms_opt(23, 59, 59)
                            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                        if let (Some(start_dt), Some(end_dt)) = (start_dt, end_dt) {
                            fetch_sales_report(start_dt, end_dt).await.map_err(|e| e.to_string())
                        } else {
                            Err("Invalid date format".to_string())
                        }
                    } else {
                        Err("Please select valid start and end dates".to_string())
                    }
                }
                _ => Err("Invalid report type".to_string()),
            };

            match result {
                Ok(report_data) => { set_report.set(Some(report_data)); set_error.set(None); }
                Err(e) => { set_error.set(Some(e)); set_report.set(None); }
            }
            set_loading.set(false);
        });
    };

    Effect::new(move || { load_report("daily".to_string()); });

    view! {
        <div class="reports-page">
            <h2>"Sales Reports"</h2>

            <div class="report-controls">
                <div class="report-type-selector">
                    <button
                        class=move || if report_type.get() == "daily" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| { set_report_type.set("daily".to_string()); load_report("daily".to_string()); }
                    >"Today"</button>
                    <button
                        class=move || if report_type.get() == "monthly" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| { set_report_type.set("monthly".to_string()); load_report("monthly".to_string()); }
                    >"Last 30 Days"</button>
                    <button
                        class=move || if report_type.get() == "custom" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| set_report_type.set("custom".to_string())
                    >"Custom Range"</button>
                </div>

                <Show when=move || report_type.get() == "custom" fallback=|| ()>
                    <div class="date-range-selector">
                        <div class="form-group">
                            <label>"Start Date"</label>
                            <input type="date" value=move || start_date.get() on:input=move |ev| set_start_date.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>"End Date"</label>
                            <input type="date" value=move || end_date.get() on:input=move |ev| set_end_date.set(event_target_value(&ev)) />
                        </div>
                        <button class="btn-primary" on:click=move |_| load_report("custom".to_string())>"Generate Report"</button>
                    </div>
                </Show>
            </div>

            <Show when=move || loading.get() fallback=|| ()>
                <div class="loading">"Loading report..."</div>
            </Show>

            <Show when=move || error.get().is_some() fallback=|| ()>
                <div class="error-message">"Error: "{move || error.get().unwrap_or_default()}</div>
            </Show>

            <Show when=move || report.get().is_some() && !loading.get() fallback=|| ()>
                {move || {
                    report.get().map(|report_data| {
                        view! {
                            <div class="report-content">
                                <div class="report-header">
                                    <h3>"Report Period"</h3>
                                    <p>
                                        {report_data.start_date.format("%Y-%m-%d").to_string()}
                                        " to "
                                        {report_data.end_date.format("%Y-%m-%d").to_string()}
                                    </p>
                                </div>

                                <div class="summary-cards">
                                    <div class="summary-card"><h4>"Total Revenue"</h4><div class="summary-value">{format!("{} {:.2}", CURRENCY_SYMBOL, report_data.summary.total_revenue)}</div></div>
                                    <div class="summary-card"><h4>"Items Sold"</h4><div class="summary-value">{report_data.summary.total_items_sold.to_string()}</div></div>
                                    <div class="summary-card"><h4>"Transactions"</h4><div class="summary-value">{report_data.summary.total_transactions.to_string()}</div></div>
                                    <div class="summary-card"><h4>"Avg Transaction"</h4><div class="summary-value">{format!("{} {:.2}", CURRENCY_SYMBOL, report_data.summary.average_transaction_value)}</div></div>
                                </div>

                                <div class="report-highlights">
                                    {report_data.summary.top_selling_item.as_ref().map(|item| view! { <div class="highlight"><strong>"Top Selling Item: "</strong>{item.clone()}</div> })}
                                    {report_data.summary.top_revenue_item.as_ref().map(|item| view! { <div class="highlight"><strong>"Top Revenue Item: "</strong>{item.clone()}</div> })}
                                </div>

                                <h3>"Sales by Item"</h3>
                                {if report_data.items.is_empty() {
                                    view! { <p>"No sales data for this period"</p> }.into_any()
                                } else {
                                    let items = report_data.items.clone();
                                    let total_items = report_data.summary.total_items_sold;
                                    let total_revenue = report_data.summary.total_revenue;
                                    let total_transactions = report_data.summary.total_transactions;
                                    view! {
                                        <table class="data-table">
                                            <thead><tr><th>"Item"</th><th>"Category"</th><th>"Quantity Sold"</th><th>"Revenue"</th><th>"Avg Price"</th><th>"Transactions"</th></tr></thead>
                                            <tbody>
                                                <For each=move || items.clone() key=|item| item.item_id let:item>
                                                    <tr>
                                                        <td>{item.item_name.clone()}</td>
                                                        <td>{item.category_name.clone()}</td>
                                                        <td>{item.quantity_sold.to_string()}</td>
                                                        <td>{format!("{} {:.2}", CURRENCY_SYMBOL, item.total_revenue)}</td>
                                                        <td>{format!("{} {:.2}", CURRENCY_SYMBOL, item.average_price)}</td>
                                                        <td>{item.transaction_count.to_string()}</td>
                                                    </tr>
                                                </For>
                                            </tbody>
                                            <tfoot>
                                                <tr class="table-footer">
                                                    <td colspan="2"><strong>"Total"</strong></td>
                                                    <td><strong>{total_items.to_string()}</strong></td>
                                                    <td><strong>{format!("{} {:.2}", CURRENCY_SYMBOL, total_revenue)}</strong></td>
                                                    <td>"-"</td>
                                                    <td><strong>{total_transactions.to_string()}</strong></td>
                                                </tr>
                                            </tfoot>
                                        </table>
                                    }.into_any()
                                }}
                            </div>
                        }
                    })
                }}
            </Show>
        </div>
    }
}
