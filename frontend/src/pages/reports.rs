use chrono::{DateTime, Utc};
use leptos::prelude::*;

use crate::i18n::I18n;
use crate::models::*;
use crate::server_fns::*;

const CURRENCY_SYMBOL: &str = "€";

fn redirect_to_login_reports() {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = web_sys::window().unwrap().location().set_href("/login");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_csv_download(_csv: &str, _filename: &str) {}

#[cfg(target_arch = "wasm32")]
fn trigger_csv_download(csv: &str, filename: &str) {
    use wasm_bindgen::prelude::*;
    let doc = leptos::prelude::document();
    let a: web_sys::HtmlAnchorElement = doc.create_element("a").unwrap().unchecked_into();
    let encoded = format!("data:text/csv;charset=utf-8,{}", js_sys::encode_uri_component(csv));
    a.set_href(&encoded);
    a.set_download(filename);
    a.click();
}

#[component]
pub fn ReportsPage() -> impl IntoView {
    let i18n = expect_context::<RwSignal<I18n>>();

    let (authorized, set_authorized) = signal(false);
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            match get_current_user().await {
                Ok(Some(u)) if u.role == "admin" => set_authorized.set(true),
                _ => redirect_to_login_reports(),
            }
        });
    });

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
        <Show when=move || authorized.get() fallback=move || view! { <div class="loading">{move || i18n.get().t("general.loading")}</div> }>
        <div class="reports-page">
            <h2>{move || i18n.get().t("reports.title")}</h2>

            <div class="report-controls">
                <div class="report-type-selector">
                    <button
                        class=move || if report_type.get() == "daily" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| { set_report_type.set("daily".to_string()); load_report("daily".to_string()); }
                    >{move || i18n.get().t("reports.today")}</button>
                    <button
                        class=move || if report_type.get() == "monthly" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| { set_report_type.set("monthly".to_string()); load_report("monthly".to_string()); }
                    >{move || i18n.get().t("reports.monthly")}</button>
                    <button
                        class=move || if report_type.get() == "custom" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| set_report_type.set("custom".to_string())
                    >{move || i18n.get().t("reports.custom")}</button>
                </div>

                <Show when=move || report_type.get() == "custom" fallback=|| ()>
                    <div class="date-range-selector">
                        <div class="form-group">
                            <label>{move || i18n.get().t("reports.start_date")}</label>
                            <input type="date" value=move || start_date.get() on:input=move |ev| set_start_date.set(event_target_value(&ev)) />
                        </div>
                        <div class="form-group">
                            <label>{move || i18n.get().t("reports.end_date")}</label>
                            <input type="date" value=move || end_date.get() on:input=move |ev| set_end_date.set(event_target_value(&ev)) />
                        </div>
                        <button class="btn-primary" on:click=move |_| load_report("custom".to_string())>{move || i18n.get().t("reports.generate")}</button>
                    </div>
                </Show>
            </div>

            <Show when=move || report.get().is_some() && !loading.get() fallback=|| ()>
                <button class="btn-primary" style="margin-bottom: 1rem;" on:click=move |_| {
                    if let Some(r) = report.get() {
                        let sd = r.start_date;
                        let ed = r.end_date;
                        leptos::task::spawn_local(async move {
                            if let Ok(csv) = export_report_csv(sd, ed).await {
                                trigger_csv_download(&csv, "sales_report.csv");
                            }
                        });
                    }
                }>{move || i18n.get().t("reports.export_csv")}</button>
            </Show>

            <Show when=move || loading.get() fallback=|| ()>
                <div class="loading">{move || i18n.get().t("reports.loading")}</div>
            </Show>

            <Show when=move || error.get().is_some() fallback=|| ()>
                <div class="error-message">{move || i18n.get().t("reports.error")}{move || error.get().unwrap_or_default()}</div>
            </Show>

            <Show when=move || report.get().is_some() && !loading.get() fallback=|| ()>
                {move || {
                    report.get().map(|report_data| {
                        view! {
                            <div class="report-content">
                                <div class="report-header">
                                    <h3>{i18n.get().t("reports.period")}</h3>
                                    <p>
                                        {report_data.start_date.format("%Y-%m-%d").to_string()}
                                        {i18n.get().t("reports.to")}
                                        {report_data.end_date.format("%Y-%m-%d").to_string()}
                                    </p>
                                </div>

                                <div class="summary-cards">
                                    <div class="summary-card"><h4>{i18n.get().t("reports.total_revenue")}</h4><div class="summary-value">{format!("{} {:.2}", CURRENCY_SYMBOL, report_data.summary.total_revenue)}</div></div>
                                    <div class="summary-card"><h4>{i18n.get().t("reports.items_sold")}</h4><div class="summary-value">{report_data.summary.total_items_sold.to_string()}</div></div>
                                    <div class="summary-card"><h4>{i18n.get().t("reports.transactions")}</h4><div class="summary-value">{report_data.summary.total_transactions.to_string()}</div></div>
                                    <div class="summary-card"><h4>{i18n.get().t("reports.avg_transaction")}</h4><div class="summary-value">{format!("{} {:.2}", CURRENCY_SYMBOL, report_data.summary.average_transaction_value)}</div></div>
                                </div>

                                <div class="report-highlights">
                                    {report_data.summary.top_selling_item.as_ref().map(|item| view! { <div class="highlight"><strong>{i18n.get().t("reports.top_selling")}</strong>{item.clone()}</div> })}
                                    {report_data.summary.top_revenue_item.as_ref().map(|item| view! { <div class="highlight"><strong>{i18n.get().t("reports.top_revenue")}</strong>{item.clone()}</div> })}
                                </div>

                                <h3>{i18n.get().t("reports.sales_by_item")}</h3>
                                {if report_data.items.is_empty() {
                                    view! { <p>{i18n.get().t("reports.no_data")}</p> }.into_any()
                                } else {
                                    let items = report_data.items.clone();
                                    let total_items = report_data.summary.total_items_sold;
                                    let total_revenue = report_data.summary.total_revenue;
                                    let total_transactions = report_data.summary.total_transactions;
                                    view! {
                                        <table class="data-table">
                                            <thead><tr><th>{i18n.get().t("reports.item")}</th><th>{i18n.get().t("reports.category")}</th><th>{i18n.get().t("reports.quantity_sold")}</th><th>{i18n.get().t("reports.revenue")}</th><th>{i18n.get().t("reports.avg_price")}</th><th>{i18n.get().t("reports.transactions")}</th></tr></thead>
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
                                                    <td colspan="2"><strong>{i18n.get().t("reports.total")}</strong></td>
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
        </Show>
    }
}
