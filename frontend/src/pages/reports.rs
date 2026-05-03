use chrono::{DateTime, Utc};
use leptos::prelude::*;

use crate::i18n::I18n;
use crate::models::*;
use crate::server_fns::*;

const CHART_PALETTE: [&str; 10] = [
    "#3b82f6", "#10b981", "#f59e0b", "#ef4444", "#8b5cf6",
    "#06b6d4", "#ec4899", "#84cc16", "#f97316", "#6366f1",
];

#[derive(Clone)]
struct PieSlice {
    label: String,
    value: f64,
    color: String,
}

fn pie_chart_view(
    slices: Vec<PieSlice>,
    donut: bool,
    center_label: Option<(String, String)>, // (top line, bottom line)
    value_fmt: impl Fn(f64) -> String + 'static,
) -> impl IntoView {
    let total: f64 = slices.iter().map(|s| s.value).sum();
    let cx = 110.0_f64;
    let cy = 110.0_f64;
    let r_outer = 100.0_f64;
    let r_inner = if donut { 60.0_f64 } else { 0.0_f64 };

    let mut paths: Vec<(String, String, String)> = Vec::new();
    let mut acc = 0.0_f64;
    if total > 0.0 {
        for s in &slices {
            let frac = s.value / total;
            let start = acc;
            let end = acc + frac;
            acc = end;
            let a0 = -std::f64::consts::FRAC_PI_2 + start * std::f64::consts::TAU;
            let a1 = -std::f64::consts::FRAC_PI_2 + end * std::f64::consts::TAU;
            let large = if frac > 0.5 { 1 } else { 0 };

            let d = if frac >= 0.999 {
                if donut {
                    // Outer ring + inner cutout (using even-odd fill).
                    format!(
                        "M {cx:.2} {ot:.2} A {ro:.2} {ro:.2} 0 1 1 {cx:.2} {ob:.2} A {ro:.2} {ro:.2} 0 1 1 {cx:.2} {ot:.2} Z M {cx:.2} {it:.2} A {ri:.2} {ri:.2} 0 1 0 {cx:.2} {ib:.2} A {ri:.2} {ri:.2} 0 1 0 {cx:.2} {it:.2} Z",
                        cx = cx,
                        ot = cy - r_outer,
                        ob = cy + r_outer,
                        it = cy - r_inner,
                        ib = cy + r_inner,
                        ro = r_outer,
                        ri = r_inner,
                    )
                } else {
                    format!(
                        "M {cx:.2} {ot:.2} A {ro:.2} {ro:.2} 0 1 1 {cx:.2} {ob:.2} A {ro:.2} {ro:.2} 0 1 1 {cx:.2} {ot:.2} Z",
                        cx = cx,
                        ot = cy - r_outer,
                        ob = cy + r_outer,
                        ro = r_outer,
                    )
                }
            } else {
                let xo0 = cx + r_outer * a0.cos();
                let yo0 = cy + r_outer * a0.sin();
                let xo1 = cx + r_outer * a1.cos();
                let yo1 = cy + r_outer * a1.sin();
                if donut {
                    let xi0 = cx + r_inner * a0.cos();
                    let yi0 = cy + r_inner * a0.sin();
                    let xi1 = cx + r_inner * a1.cos();
                    let yi1 = cy + r_inner * a1.sin();
                    format!(
                        "M {xo0:.2} {yo0:.2} A {ro:.2} {ro:.2} 0 {large} 1 {xo1:.2} {yo1:.2} L {xi1:.2} {yi1:.2} A {ri:.2} {ri:.2} 0 {large} 0 {xi0:.2} {yi0:.2} Z",
                        ro = r_outer,
                        ri = r_inner,
                    )
                } else {
                    format!(
                        "M {cx:.2} {cy:.2} L {xo0:.2} {yo0:.2} A {ro:.2} {ro:.2} 0 {large} 1 {xo1:.2} {yo1:.2} Z",
                        ro = r_outer,
                    )
                }
            };
            paths.push((d, s.color.clone(), s.label.clone()));
        }
    }

    let legend_items: Vec<_> = slices
        .iter()
        .map(|s| {
            let pct = if total > 0.0 { s.value / total * 100.0 } else { 0.0 };
            (s.label.clone(), s.color.clone(), value_fmt(s.value), pct)
        })
        .collect();

    let center = center_label.clone();

    view! {
        <div class="chart-pie-container">
            <svg class="chart-pie" viewBox="0 0 220 220" xmlns="http://www.w3.org/2000/svg">
                {paths.into_iter().map(|(d, color, label)| {
                    view! {
                        <path d=d fill=color stroke="white" stroke-width="2" fill-rule="evenodd">
                            <title>{label}</title>
                        </path>
                    }
                }).collect_view()}
                {move || if total <= 0.0 {
                    Some(view! {
                        <circle cx="110" cy="110" r="100" fill="#e5e7eb" />
                    })
                } else { None }}
                {move || if total <= 0.0 && donut {
                    Some(view! {
                        <circle cx="110" cy="110" r="60" fill="white" />
                    })
                } else { None }}
                {center.map(|(top, bot)| view! {
                    <g>
                        <text x="110" y="105" text-anchor="middle" font-size="20" font-weight="700" fill="#111827">{top}</text>
                        <text x="110" y="128" text-anchor="middle" font-size="11" fill="#6b7280">{bot}</text>
                    </g>
                })}
            </svg>
            <ul class="chart-legend">
                {legend_items.into_iter().map(|(label, color, val, pct)| {
                    view! {
                        <li>
                            <span class="legend-swatch" style=format!("background:{}", color)></span>
                            <span class="legend-label">{label}</span>
                            <span class="legend-value">{val}" ("{format!("{:.1}", pct)}"%)"</span>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

fn stacked_bar_chart_view(ts: ItemSalesTimeseries) -> impl IntoView {
    let n_buckets = ts.buckets.len();
    let n_items = ts.item_names.len();

    // SVG dimensions
    let width = 900.0_f64;
    let height = 360.0_f64;
    let margin_left = 50.0_f64;
    let margin_right = 20.0_f64;
    let margin_top = 20.0_f64;
    let margin_bottom = 50.0_f64;
    let plot_w = width - margin_left - margin_right;
    let plot_h = height - margin_top - margin_bottom;

    // Compute max stacked total across buckets
    let max_total: i64 = ts
        .buckets
        .iter()
        .map(|b| b.quantities.iter().sum::<i64>())
        .max()
        .unwrap_or(0)
        .max(1);

    // Round max up to a "nice" tick value for the y axis
    let nice_max = nice_ceiling(max_total);

    let bar_w = if n_buckets > 0 {
        (plot_w / n_buckets as f64) * 0.7
    } else {
        0.0
    };
    let bar_step = if n_buckets > 0 {
        plot_w / n_buckets as f64
    } else {
        0.0
    };

    // Decide x-axis label stride to avoid overlap
    let label_stride = if n_buckets <= 12 {
        1
    } else if n_buckets <= 30 {
        2
    } else if n_buckets <= 60 {
        5
    } else {
        (n_buckets / 12).max(1)
    };

    // y-axis ticks
    let n_ticks = 5;
    let tick_values: Vec<i64> = (0..=n_ticks)
        .map(|i| nice_max * i as i64 / n_ticks as i64)
        .collect();

    let segments: Vec<(f64, f64, f64, f64, String, String)> = {
        let mut out = Vec::new();
        for (bi, bucket) in ts.buckets.iter().enumerate() {
            let x = margin_left + bar_step * bi as f64 + (bar_step - bar_w) / 2.0;
            let mut y_acc = margin_top + plot_h;
            for (ii, &qty) in bucket.quantities.iter().enumerate() {
                if qty <= 0 {
                    continue;
                }
                let h = qty as f64 / nice_max as f64 * plot_h;
                let y = y_acc - h;
                let color = CHART_PALETTE[ii % CHART_PALETTE.len()].to_string();
                let title = format!(
                    "{} — {}: {}",
                    bucket.label,
                    ts.item_names.get(ii).cloned().unwrap_or_default(),
                    qty
                );
                out.push((x, y, bar_w, h, color, title));
                y_acc = y;
            }
        }
        out
    };

    let x_labels: Vec<(f64, String)> = ts
        .buckets
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            if i % label_stride != 0 {
                return None;
            }
            let x = margin_left + bar_step * i as f64 + bar_step / 2.0;
            Some((x, b.label.clone()))
        })
        .collect();

    let legend: Vec<(String, String)> = ts
        .item_names
        .iter()
        .enumerate()
        .map(|(i, name)| (CHART_PALETTE[i % CHART_PALETTE.len()].to_string(), name.clone()))
        .collect();

    view! {
        <div class="chart-stacked-container">
            <svg class="chart-stacked" viewBox=format!("0 0 {} {}", width, height) preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg">
                // y-axis grid + ticks
                {tick_values.iter().map(|&v| {
                    let y = margin_top + plot_h - (v as f64 / nice_max as f64 * plot_h);
                    view! {
                        <g>
                            <line x1=margin_left y1=y x2={margin_left + plot_w} y2=y stroke="#e5e7eb" stroke-width="1" />
                            <text x={margin_left - 6.0} y={y + 4.0} text-anchor="end" font-size="11" fill="#6b7280">{v.to_string()}</text>
                        </g>
                    }
                }).collect_view()}
                // x-axis baseline
                <line x1=margin_left y1={margin_top + plot_h} x2={margin_left + plot_w} y2={margin_top + plot_h} stroke="#9ca3af" stroke-width="1" />
                // Bars
                {segments.into_iter().map(|(x, y, w, h, color, title)| {
                    view! {
                        <rect x=x y=y width=w height=h fill=color>
                            <title>{title}</title>
                        </rect>
                    }
                }).collect_view()}
                // X labels
                {x_labels.into_iter().map(|(x, label)| {
                    view! {
                        <text x=x y={margin_top + plot_h + 18.0} text-anchor="middle" font-size="11" fill="#374151">{label}</text>
                    }
                }).collect_view()}
                {move || if n_items == 0 {
                    Some(view! {
                        <text x={width/2.0} y={height/2.0} text-anchor="middle" font-size="14" fill="#9ca3af">"No data"</text>
                    })
                } else { None }}
            </svg>
            <ul class="chart-legend chart-legend-horizontal">
                {legend.into_iter().map(|(color, name)| {
                    view! {
                        <li>
                            <span class="legend-swatch" style=format!("background:{}", color)></span>
                            <span class="legend-label">{name}</span>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

fn line_chart_view(
    points: Vec<(String, f64)>,
    line_color: String,
    fill_color: String,
    y_fmt: impl Fn(f64) -> String + 'static,
) -> impl IntoView {
    let width = 900.0_f64;
    let height = 280.0_f64;
    let margin_left = 70.0_f64;
    let margin_right = 20.0_f64;
    let margin_top = 20.0_f64;
    let margin_bottom = 50.0_f64;
    let plot_w = width - margin_left - margin_right;
    let plot_h = height - margin_top - margin_bottom;

    let n = points.len();
    let max_y = points
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let nice_max = nice_ceiling_f(max_y);

    let label_stride = if n <= 12 {
        1
    } else if n <= 30 {
        2
    } else if n <= 60 {
        5
    } else {
        (n / 12).max(1)
    };

    let x_at = |i: usize| -> f64 {
        if n <= 1 {
            margin_left + plot_w / 2.0
        } else {
            margin_left + (i as f64) * (plot_w / (n.saturating_sub(1)).max(1) as f64)
        }
    };
    let y_at = |v: f64| -> f64 { margin_top + plot_h - (v / nice_max * plot_h) };

    let line_path = if n == 0 {
        String::new()
    } else {
        let mut s = String::new();
        for (i, (_, v)) in points.iter().enumerate() {
            let x = x_at(i);
            let y = y_at(*v);
            if i == 0 {
                s.push_str(&format!("M {:.2} {:.2}", x, y));
            } else {
                s.push_str(&format!(" L {:.2} {:.2}", x, y));
            }
        }
        s
    };

    let area_path = if n == 0 {
        String::new()
    } else {
        let mut s = line_path.clone();
        let last_x = x_at(n - 1);
        let first_x = x_at(0);
        let baseline_y = margin_top + plot_h;
        s.push_str(&format!(
            " L {:.2} {:.2} L {:.2} {:.2} Z",
            last_x, baseline_y, first_x, baseline_y
        ));
        s
    };

    let n_ticks = 5;
    let tick_values: Vec<f64> = (0..=n_ticks)
        .map(|i| nice_max * i as f64 / n_ticks as f64)
        .collect();

    let x_labels: Vec<(f64, String)> = points
        .iter()
        .enumerate()
        .filter_map(|(i, (label, _))| {
            if i % label_stride != 0 && i + 1 != n {
                return None;
            }
            Some((x_at(i), label.clone()))
        })
        .collect();

    let dot_points: Vec<(f64, f64, String)> = points
        .iter()
        .enumerate()
        .map(|(i, (label, v))| (x_at(i), y_at(*v), format!("{}: {}", label, y_fmt(*v))))
        .collect();

    view! {
        <div class="chart-stacked-container">
            <svg class="chart-stacked" viewBox=format!("0 0 {} {}", width, height) preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg">
                {tick_values.iter().map(|&v| {
                    let y = y_at(v);
                    let label = y_fmt(v);
                    view! {
                        <g>
                            <line x1=margin_left y1=y x2={margin_left + plot_w} y2=y stroke="#e5e7eb" stroke-width="1" />
                            <text x={margin_left - 6.0} y={y + 4.0} text-anchor="end" font-size="11" fill="#6b7280">{label}</text>
                        </g>
                    }
                }).collect_view()}
                <line x1=margin_left y1={margin_top + plot_h} x2={margin_left + plot_w} y2={margin_top + plot_h} stroke="#9ca3af" stroke-width="1" />
                <path d=area_path fill=fill_color stroke="none" />
                <path d=line_path fill="none" stroke=line_color.clone() stroke-width="2" />
                {dot_points.into_iter().map(|(x, y, title)| {
                    view! {
                        <circle cx=x cy=y r="3" fill=line_color.clone() stroke="white" stroke-width="1.5">
                            <title>{title}</title>
                        </circle>
                    }
                }).collect_view()}
                {x_labels.into_iter().map(|(x, label)| {
                    view! {
                        <text x=x y={margin_top + plot_h + 18.0} text-anchor="middle" font-size="11" fill="#374151">{label}</text>
                    }
                }).collect_view()}
                {move || if n == 0 {
                    Some(view! {
                        <text x={width/2.0} y={height/2.0} text-anchor="middle" font-size="14" fill="#9ca3af">"No data"</text>
                    })
                } else { None }}
            </svg>
        </div>
    }
}

fn nice_ceiling_f(n: f64) -> f64 {
    if n <= 0.0 { return 1.0; }
    let mag = 10.0_f64.powf(n.log10().floor());
    let leading = n / mag;
    let nice_leading = if leading <= 1.0 { 1.0 }
        else if leading <= 2.0 { 2.0 }
        else if leading <= 5.0 { 5.0 }
        else { 10.0 };
    nice_leading * mag
}

fn build_top_slices<T>(
    items: &[&T],
    top: usize,
    value_of: impl Fn(&T) -> f64,
    name_of: impl Fn(&T) -> String,
) -> Vec<PieSlice> {
    let mut slices: Vec<PieSlice> = items
        .iter()
        .take(top)
        .enumerate()
        .map(|(i, it)| PieSlice {
            label: name_of(it),
            value: value_of(it),
            color: CHART_PALETTE[i % CHART_PALETTE.len()].to_string(),
        })
        .collect();
    let rest: f64 = items.iter().skip(top).map(|it| value_of(it)).sum();
    if rest > 0.0 {
        slices.push(PieSlice {
            label: "Other".to_string(),
            value: rest,
            color: "#9ca3af".to_string(),
        });
    }
    slices.into_iter().filter(|s| s.value > 0.0).collect()
}

fn nice_ceiling(n: i64) -> i64 {
    if n <= 0 { return 1; }
    let mag = 10_i64.pow((n as f64).log10().floor() as u32);
    let leading = n as f64 / mag as f64;
    let nice_leading = if leading <= 1.0 { 1 }
        else if leading <= 2.0 { 2 }
        else if leading <= 5.0 { 5 }
        else { 10 };
    nice_leading * mag
}

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
    let currency = expect_context::<RwSignal<String>>();

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
    let (timeseries, set_timeseries) = signal(Option::<ItemSalesTimeseries>::None);
    let (revenue_ts, set_revenue_ts) = signal(Option::<RevenueTimeseries>::None);
    let (basket_dist, set_basket_dist) = signal(Option::<BasketSizeDistribution>::None);
    let (payment, set_payment) = signal(Option::<PaymentAnalysis>::None);
    let (under_items, set_under_items) = signal(Vec::<UnderperformingItem>::new());
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
                Ok(report_data) => {
                    let sd = report_data.start_date;
                    let ed = report_data.end_date;
                    set_report.set(Some(report_data));
                    set_error.set(None);
                    match fetch_item_sales_timeseries(sd, ed, 10).await {
                        Ok(ts) => set_timeseries.set(Some(ts)),
                        Err(_) => set_timeseries.set(None),
                    }
                    match fetch_revenue_timeseries(sd, ed).await {
                        Ok(rt) => set_revenue_ts.set(Some(rt)),
                        Err(_) => set_revenue_ts.set(None),
                    }
                    match fetch_basket_size_distribution(sd, ed).await {
                        Ok(b) => set_basket_dist.set(Some(b)),
                        Err(_) => set_basket_dist.set(None),
                    }
                    match fetch_payment_analysis(sd, ed).await {
                        Ok(p) => set_payment.set(Some(p)),
                        Err(_) => set_payment.set(None),
                    }
                    match fetch_underperforming_items(sd, ed, 50).await {
                        Ok(u) => set_under_items.set(u),
                        Err(_) => set_under_items.set(Vec::new()),
                    }
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_report.set(None);
                    set_timeseries.set(None);
                    set_revenue_ts.set(None);
                    set_basket_dist.set(None);
                    set_payment.set(None);
                    set_under_items.set(Vec::new());
                }
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
                                    <div class="summary-card"><h4>{i18n.get().t("reports.total_revenue")}</h4><div class="summary-value">{format!("{} {:.2}", &currency.get(), report_data.summary.total_revenue)}</div></div>
                                    <div class="summary-card"><h4>{i18n.get().t("reports.items_sold")}</h4><div class="summary-value">{report_data.summary.total_items_sold.to_string()}</div></div>
                                    <div class="summary-card"><h4>{i18n.get().t("reports.transactions")}</h4><div class="summary-value">{report_data.summary.total_transactions.to_string()}</div></div>
                                    <div class="summary-card"><h4>{i18n.get().t("reports.avg_transaction")}</h4><div class="summary-value">{format!("{} {:.2}", &currency.get(), report_data.summary.average_transaction_value)}</div></div>
                                </div>

                                <div class="report-highlights">
                                    {report_data.summary.top_selling_item.as_ref().map(|item| view! { <div class="highlight"><strong>{i18n.get().t("reports.top_selling")}</strong>{item.clone()}</div> })}
                                    {report_data.summary.top_revenue_item.as_ref().map(|item| view! { <div class="highlight"><strong>{i18n.get().t("reports.top_revenue")}</strong>{item.clone()}</div> })}
                                </div>

                                {if report_data.items.is_empty() {
                                    view! { <></> }.into_any()
                                } else {
                                    let mut by_qty: Vec<&ItemSalesReport> = report_data.items.iter().collect();
                                    by_qty.sort_by(|a, b| b.quantity_sold.cmp(&a.quantity_sold));
                                    let qty_slices = build_top_slices(&by_qty, 8, |it| it.quantity_sold as f64, |it| it.item_name.clone());
                                    // items already arrives sorted by revenue desc
                                    let by_rev: Vec<&ItemSalesReport> = report_data.items.iter().collect();
                                    let rev_slices = build_top_slices(&by_rev, 8, |it| it.total_revenue, |it| it.item_name.clone());
                                    let cur_for_pie = currency.get();
                                    view! {
                                        <div class="charts-row">
                                            <div class="chart-card">
                                                <h3>{i18n.get().t("reports.chart_top_quantity")}</h3>
                                                {pie_chart_view(qty_slices, false, None, |v| format!("{}", v as i64))}
                                            </div>
                                            <div class="chart-card">
                                                <h3>{i18n.get().t("reports.chart_top_revenue")}</h3>
                                                {pie_chart_view(rev_slices, false, None, move |v| format!("{} {:.2}", cur_for_pie, v))}
                                            </div>
                                        </div>
                                    }.into_any()
                                }}

                                // Category breakdown donut + items-per-basket donut
                                {
                                    let cat_slices: Vec<PieSlice> = if report_data.items.is_empty() {
                                        Vec::new()
                                    } else {
                                        let mut agg: Vec<(String, f64)> = Vec::new();
                                        for it in &report_data.items {
                                            if let Some(entry) = agg.iter_mut().find(|(n, _)| *n == it.category_name) {
                                                entry.1 += it.total_revenue;
                                            } else {
                                                agg.push((it.category_name.clone(), it.total_revenue));
                                            }
                                        }
                                        agg.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                                        let refs: Vec<&(String, f64)> = agg.iter().collect();
                                        build_top_slices(&refs, 8, |t| t.1, |t| t.0.clone())
                                    };
                                    let cat_total = report_data.summary.total_revenue;
                                    let cur_for_cat = currency.get();
                                    let cur_for_cat_fmt = currency.get();
                                    let cat_center = if cat_total > 0.0 {
                                        Some((format!("{} {:.0}", cur_for_cat, cat_total), i18n.get().t("reports.total_revenue")))
                                    } else { None };

                                    let basket_view = move || basket_dist.get().map(|bd| {
                                        let slices: Vec<PieSlice> = bd.buckets.iter().enumerate().map(|(i, b)| PieSlice {
                                            label: b.label.clone(),
                                            value: b.count as f64,
                                            color: CHART_PALETTE[i % CHART_PALETTE.len()].to_string(),
                                        }).filter(|s| s.value > 0.0).collect();
                                        let center = if bd.total_transactions > 0 {
                                            Some((
                                                format!("{:.1}", bd.average_items),
                                                "avg items".to_string(),
                                            ))
                                        } else { None };
                                        view! {
                                            <div class="chart-card">
                                                <h3>{i18n.get().t("reports.chart_basket_size")}</h3>
                                                {pie_chart_view(slices, true, center, |v| format!("{}", v as i64))}
                                            </div>
                                        }
                                    });

                                    view! {
                                        <div class="charts-row">
                                            <div class="chart-card">
                                                <h3>{i18n.get().t("reports.chart_category_breakdown")}</h3>
                                                {pie_chart_view(cat_slices, true, cat_center, move |v| format!("{} {:.2}", cur_for_cat_fmt, v))}
                                            </div>
                                            {basket_view}
                                        </div>
                                    }
                                }

                                {move || revenue_ts.get().map(|rt| {
                                    let cur = currency.get();
                                    let cur_for_y = currency.get();
                                    let mut acc = 0.0_f64;
                                    let points: Vec<(String, f64)> = rt.buckets.iter().map(|b| {
                                        acc += b.revenue;
                                        (b.label.clone(), acc)
                                    }).collect();
                                    let _ = cur;
                                    view! {
                                        <div class="chart-card chart-card-wide">
                                            <h3>{i18n.get().t("reports.chart_cumulative_revenue")}</h3>
                                            {line_chart_view(points, "#3b82f6".to_string(), "rgba(59, 130, 246, 0.15)".to_string(), move |v| format!("{} {:.0}", cur_for_y, v))}
                                        </div>
                                    }
                                })}

                                {move || timeseries.get().map(|ts| {
                                    let i18n_v = i18n.get();
                                    view! {
                                        <div class="chart-card chart-card-wide">
                                            <h3>{i18n_v.t("reports.chart_top10_over_time")}</h3>
                                            {stacked_bar_chart_view(ts)}
                                        </div>
                                    }
                                })}

                                {move || payment.get().map(|p| {
                                    let cur = currency.get();
                                    let max_count = p.change_distribution.iter().map(|c| c.count).max().unwrap_or(1).max(1);
                                    let nice = nice_ceiling(max_count);
                                    let exact_pct = if p.transaction_count > 0 {
                                        p.exact_payment_count as f64 / p.transaction_count as f64 * 100.0
                                    } else { 0.0 };
                                    view! {
                                        <div class="chart-card chart-card-wide">
                                            <h3>{i18n.get().t("reports.chart_payment_analysis")}</h3>
                                            <div class="payment-stats">
                                                <div class="payment-stat">
                                                    <div class="payment-stat-label">{i18n.get().t("reports.payment_total_paid")}</div>
                                                    <div class="payment-stat-value">{format!("{} {:.2}", cur, p.total_paid)}</div>
                                                </div>
                                                <div class="payment-stat">
                                                    <div class="payment-stat-label">{i18n.get().t("reports.payment_total_change")}</div>
                                                    <div class="payment-stat-value">{format!("{} {:.2}", cur, p.total_change)}</div>
                                                </div>
                                                <div class="payment-stat">
                                                    <div class="payment-stat-label">{i18n.get().t("reports.payment_avg_change")}</div>
                                                    <div class="payment-stat-value">{format!("{} {:.2}", cur, p.average_change)}</div>
                                                </div>
                                                <div class="payment-stat">
                                                    <div class="payment-stat-label">{i18n.get().t("reports.payment_exact")}</div>
                                                    <div class="payment-stat-value">{format!("{} ({:.0}%)", p.exact_payment_count, exact_pct)}</div>
                                                </div>
                                            </div>
                                            <div class="payment-histogram">
                                                <div class="payment-histogram-title">{i18n.get().t("reports.payment_change_distribution")}</div>
                                                <div class="hist-bars">
                                                    {p.change_distribution.into_iter().map(|cb| {
                                                        let h_pct = (cb.count as f64 / nice as f64 * 100.0).max(0.0);
                                                        view! {
                                                            <div class="hist-bar-wrap">
                                                                <div class="hist-bar-area">
                                                                    <div class="hist-bar-count">{cb.count.to_string()}</div>
                                                                    <div class="hist-bar" style=format!("height: {:.1}%", h_pct)></div>
                                                                </div>
                                                                <div class="hist-bar-label">{cb.label}</div>
                                                            </div>
                                                        }
                                                    }).collect_view()}
                                                </div>
                                            </div>
                                        </div>
                                    }
                                })}

                                {move || {
                                    let items = under_items.get();
                                    if items.is_empty() {
                                        return view! { <></> }.into_any();
                                    }
                                    let cur = currency.get();
                                    view! {
                                        <div class="chart-card chart-card-wide">
                                            <h3>{i18n.get().t("reports.underperformers_title")}</h3>
                                            <p class="muted">{i18n.get().t("reports.underperformers_hint")}</p>
                                            <table class="data-table">
                                                <thead>
                                                    <tr>
                                                        <th>{i18n.get().t("reports.item")}</th>
                                                        <th>{i18n.get().t("reports.category")}</th>
                                                        <th>{i18n.get().t("reports.avg_price")}</th>
                                                        <th>{i18n.get().t("reports.created_at")}</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {items.into_iter().map(|it| {
                                                        view! {
                                                            <tr>
                                                                <td>{it.item_name}</td>
                                                                <td>{it.category_name}</td>
                                                                <td>{format!("{} {:.2}", cur, it.price)}</td>
                                                                <td>{it.created_at.format("%Y-%m-%d").to_string()}</td>
                                                            </tr>
                                                        }
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        </div>
                                    }.into_any()
                                }}

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
                                                        <td>{format!("{} {:.2}", &currency.get(), item.total_revenue)}</td>
                                                        <td>{format!("{} {:.2}", &currency.get(), item.average_price)}</td>
                                                        <td>{item.transaction_count.to_string()}</td>
                                                    </tr>
                                                </For>
                                            </tbody>
                                            <tfoot>
                                                <tr class="table-footer">
                                                    <td colspan="2"><strong>{i18n.get().t("reports.total")}</strong></td>
                                                    <td><strong>{total_items.to_string()}</strong></td>
                                                    <td><strong>{format!("{} {:.2}", &currency.get(), total_revenue)}</strong></td>
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
