use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{A, Route, Router, Routes},
    StaticSegment,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

const CURRENCY_SYMBOL: &str = "€";

// ---- Shared Models ----

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Item {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub category_id: Uuid,
    pub sku: Option<String>,
    pub in_stock: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Transaction {
    pub id: Uuid,
    pub customer_name: Option<String>,
    pub status: String,
    pub total: f64,
    pub paid_amount: Option<f64>,
    pub change_amount: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct TransactionItemDetail {
    pub id: Uuid,
    pub item_id: Uuid,
    pub item_name: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub total_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetailsResponse {
    pub transaction: Transaction,
    pub items: Vec<TransactionItemDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseTransactionResponse {
    pub transaction: Transaction,
    pub change_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct ItemSalesReport {
    pub item_id: Uuid,
    pub item_name: String,
    pub category_name: String,
    pub quantity_sold: i64,
    pub total_revenue: f64,
    pub average_price: f64,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_revenue: f64,
    pub total_items_sold: i64,
    pub total_transactions: i64,
    pub average_transaction_value: f64,
    pub top_selling_item: Option<String>,
    pub top_revenue_item: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesReport {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub items: Vec<ItemSalesReport>,
    pub summary: ReportSummary,
}

// ---- Server Helpers (SSR only) ----

#[cfg(feature = "ssr")]
fn db_err(e: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::ServerError(e.to_string())
}

#[cfg(feature = "ssr")]
fn not_found(msg: &str) -> ServerFnError {
    ServerFnError::ServerError(msg.to_string())
}

#[cfg(feature = "ssr")]
async fn update_transaction_total_db(
    pool: &sqlx::SqlitePool,
    transaction_id: Uuid,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "UPDATE transactions SET total = (
            SELECT COALESCE(SUM(total_price), 0) FROM transaction_items WHERE transaction_id = ?
        ), updated_at = ? WHERE id = ?",
    )
    .bind(transaction_id)
    .bind(Utc::now())
    .bind(transaction_id)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[cfg(feature = "ssr")]
async fn generate_sales_report_db(
    pool: &sqlx::SqlitePool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<SalesReport, ServerFnError> {
    if end_date <= start_date {
        return Err(not_found("End date must be after start date"));
    }

    let items = sqlx::query_as::<_, ItemSalesReport>(
        "SELECT i.id as item_id, i.name as item_name, c.name as category_name,
         SUM(ti.quantity) as quantity_sold, SUM(ti.total_price) as total_revenue,
         AVG(ti.unit_price) as average_price, COUNT(DISTINCT ti.transaction_id) as transaction_count
         FROM transaction_items ti
         JOIN items i ON ti.item_id = i.id
         JOIN categories c ON i.category_id = c.id
         JOIN transactions t ON ti.transaction_id = t.id
         WHERE t.status = 'closed' AND t.closed_at >= ? AND t.closed_at < ?
         GROUP BY i.id, i.name, c.name ORDER BY total_revenue DESC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total_revenue: f64 = items.iter().map(|i| i.total_revenue).sum();
    let total_items_sold: i64 = items.iter().map(|i| i.quantity_sold).sum();

    let transaction_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT id) FROM transactions
         WHERE status = 'closed' AND closed_at >= ? AND closed_at < ?",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    let average_transaction_value = if transaction_count > 0 {
        total_revenue / transaction_count as f64
    } else {
        0.0
    };

    let top_selling_item = items
        .iter()
        .max_by_key(|i| i.quantity_sold)
        .map(|i| i.item_name.clone());

    let top_revenue_item = items
        .iter()
        .max_by(|a, b| a.total_revenue.partial_cmp(&b.total_revenue).unwrap())
        .map(|i| i.item_name.clone());

    Ok(SalesReport {
        start_date,
        end_date,
        items,
        summary: ReportSummary {
            total_revenue,
            total_items_sold,
            total_transactions: transaction_count,
            average_transaction_value,
            top_selling_item,
            top_revenue_item,
        },
    })
}

// ---- Server Functions ----

// Categories

#[server]
pub async fn fetch_categories() -> Result<Vec<Category>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY name")
        .fetch_all(&pool)
        .await
        .map_err(db_err)?;
    Ok(categories)
}

#[server]
pub async fn create_category(
    name: String,
    description: Option<String>,
) -> Result<Category, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let id = Uuid::new_v4();
    let now = Utc::now();
    let category = sqlx::query_as::<_, Category>(
        "INSERT INTO categories (id, name, description, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(&name)
    .bind(&description)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(category)
}

#[server]
pub async fn update_category(
    id: Uuid,
    name: Option<String>,
    description: Option<String>,
) -> Result<Category, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let mut category = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Category not found"))?;

    if let Some(n) = name {
        category.name = n;
    }
    if let Some(d) = description {
        category.description = Some(d);
    }
    category.updated_at = Utc::now();

    let updated = sqlx::query_as::<_, Category>(
        "UPDATE categories SET name = ?, description = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&category.name)
    .bind(&category.description)
    .bind(category.updated_at)
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(updated)
}

#[server]
pub async fn delete_category(id: Uuid) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let result = sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(not_found("Category not found"));
    }
    Ok(())
}

// Items

#[server]
pub async fn fetch_items() -> Result<Vec<Item>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let items = sqlx::query_as::<_, Item>("SELECT * FROM items ORDER BY name")
        .fetch_all(&pool)
        .await
        .map_err(db_err)?;
    Ok(items)
}

#[server]
pub async fn create_item(
    name: String,
    description: Option<String>,
    price: f64,
    category_id: Uuid,
    sku: Option<String>,
    in_stock: Option<bool>,
) -> Result<Item, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let id = Uuid::new_v4();
    let now = Utc::now();
    let in_stock = in_stock.unwrap_or(true);
    let item = sqlx::query_as::<_, Item>(
        "INSERT INTO items (id, name, description, price, category_id, sku, in_stock, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(&name)
    .bind(&description)
    .bind(price)
    .bind(category_id)
    .bind(&sku)
    .bind(in_stock)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(item)
}

#[server]
pub async fn update_item(
    id: Uuid,
    name: Option<String>,
    description: Option<String>,
    price: Option<f64>,
    category_id: Option<Uuid>,
    sku: Option<String>,
    in_stock: Option<bool>,
) -> Result<Item, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let mut item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Item not found"))?;

    if let Some(n) = name {
        item.name = n;
    }
    if let Some(d) = description {
        item.description = Some(d);
    }
    if let Some(p) = price {
        item.price = p;
    }
    if let Some(c) = category_id {
        item.category_id = c;
    }
    if let Some(s) = sku {
        item.sku = Some(s);
    }
    if let Some(s) = in_stock {
        item.in_stock = s;
    }
    item.updated_at = Utc::now();

    let updated = sqlx::query_as::<_, Item>(
        "UPDATE items SET name = ?, description = ?, price = ?, category_id = ?,
         sku = ?, in_stock = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&item.name)
    .bind(&item.description)
    .bind(item.price)
    .bind(item.category_id)
    .bind(&item.sku)
    .bind(item.in_stock)
    .bind(item.updated_at)
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(updated)
}

#[server]
pub async fn delete_item(id: Uuid) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let result = sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(not_found("Item not found"));
    }
    Ok(())
}

// Transactions

#[server]
pub async fn fetch_all_transactions() -> Result<Vec<Transaction>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;
    Ok(transactions)
}

#[server]
pub async fn fetch_open_transactions() -> Result<Vec<Transaction>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE status = 'open' ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;
    Ok(transactions)
}

#[server]
pub async fn fetch_transaction_details(
    id: Uuid,
) -> Result<TransactionDetailsResponse, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let transaction =
        sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE id = ?")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(db_err)?
            .ok_or_else(|| not_found("Transaction not found"))?;

    let items = sqlx::query_as::<_, TransactionItemDetail>(
        "SELECT ti.id, ti.item_id, i.name as item_name, ti.quantity,
         ti.unit_price, ti.total_price
         FROM transaction_items ti
         JOIN items i ON ti.item_id = i.id
         WHERE ti.transaction_id = ?",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    Ok(TransactionDetailsResponse { transaction, items })
}

#[server]
pub async fn create_transaction(
    customer_name: Option<String>,
) -> Result<Transaction, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let id = Uuid::new_v4();
    let now = Utc::now();
    let transaction = sqlx::query_as::<_, Transaction>(
        "INSERT INTO transactions (id, customer_name, status, total, created_at, updated_at)
         VALUES (?, ?, 'open', 0.0, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(&customer_name)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(transaction)
}

#[server]
pub async fn update_transaction_details(
    id: Uuid,
    customer_name: Option<String>,
) -> Result<Transaction, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?
    .ok_or_else(|| not_found("Transaction not found or not open"))?;

    let updated = sqlx::query_as::<_, Transaction>(
        "UPDATE transactions SET customer_name = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&customer_name)
    .bind(Utc::now())
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(updated)
}

#[server]
pub async fn add_item_to_transaction(
    transaction_id: Uuid,
    item_id: Uuid,
    quantity: i32,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    // Check transaction is open
    sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'",
    )
    .bind(transaction_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?
    .ok_or_else(|| not_found("Transaction not found or not open"))?;

    // Get item details
    let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Item not found"))?;

    if !item.in_stock {
        return Err(not_found("Item is out of stock"));
    }

    // Check if item already in transaction
    let existing_qty = sqlx::query_scalar::<_, i32>(
        "SELECT quantity FROM transaction_items WHERE transaction_id = ? AND item_id = ?",
    )
    .bind(transaction_id)
    .bind(item_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?;

    let new_quantity = existing_qty.unwrap_or(0) + quantity;

    if new_quantity <= 0 {
        sqlx::query(
            "DELETE FROM transaction_items WHERE transaction_id = ? AND item_id = ?",
        )
        .bind(transaction_id)
        .bind(item_id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    } else if existing_qty.is_some() {
        let total_price = item.price * new_quantity as f64;
        sqlx::query(
            "UPDATE transaction_items SET quantity = ?, unit_price = ?, total_price = ?
             WHERE transaction_id = ? AND item_id = ?",
        )
        .bind(new_quantity)
        .bind(item.price)
        .bind(total_price)
        .bind(transaction_id)
        .bind(item_id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    } else {
        let id = Uuid::new_v4();
        let total_price = item.price * new_quantity as f64;
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO transaction_items (id, transaction_id, item_id, quantity, unit_price, total_price, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(transaction_id)
        .bind(item_id)
        .bind(new_quantity)
        .bind(item.price)
        .bind(total_price)
        .bind(now)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    }

    update_transaction_total_db(&pool, transaction_id).await?;
    Ok(())
}

#[server]
pub async fn remove_item_from_transaction(
    transaction_id: Uuid,
    item_id: Uuid,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    // Check transaction is open
    sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'",
    )
    .bind(transaction_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?
    .ok_or_else(|| not_found("Transaction not found or not open"))?;

    let current_quantity = sqlx::query_scalar::<_, i32>(
        "SELECT quantity FROM transaction_items WHERE transaction_id = ? AND item_id = ?",
    )
    .bind(transaction_id)
    .bind(item_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?;

    if let Some(qty) = current_quantity {
        if qty > 1 {
            let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
                .bind(item_id)
                .fetch_one(&pool)
                .await
                .map_err(db_err)?;
            let new_qty = qty - 1;
            let total_price = item.price * new_qty as f64;
            sqlx::query(
                "UPDATE transaction_items SET quantity = ?, unit_price = ?, total_price = ?
                 WHERE transaction_id = ? AND item_id = ?",
            )
            .bind(new_qty)
            .bind(item.price)
            .bind(total_price)
            .bind(transaction_id)
            .bind(item_id)
            .execute(&pool)
            .await
            .map_err(db_err)?;
        } else {
            sqlx::query(
                "DELETE FROM transaction_items WHERE transaction_id = ? AND item_id = ?",
            )
            .bind(transaction_id)
            .bind(item_id)
            .execute(&pool)
            .await
            .map_err(db_err)?;
        }
    }

    update_transaction_total_db(&pool, transaction_id).await?;
    Ok(())
}

#[server]
pub async fn close_transaction(
    id: Uuid,
    paid_amount: f64,
) -> Result<CloseTransactionResponse, ServerFnError> {
    use crate::printer::{find_printer, print_receipt};

    let pool = expect_context::<sqlx::SqlitePool>();

    let transaction = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?
    .ok_or_else(|| not_found("Transaction not found or not open"))?;

    if paid_amount < transaction.total {
        return Err(not_found("Insufficient payment amount"));
    }

    let change = paid_amount - transaction.total;
    let now = Utc::now();

    let transaction = sqlx::query_as::<_, Transaction>(
        "UPDATE transactions SET status = 'closed', paid_amount = ?, change_amount = ?,
         closed_at = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(paid_amount)
    .bind(change)
    .bind(now)
    .bind(now)
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;

    // Print receipt
    if transaction.status == "closed" {
        let items = sqlx::query_as::<_, TransactionItemDetail>(
            "SELECT ti.id, ti.item_id, i.name as item_name, ti.quantity,
             ti.unit_price, ti.total_price
             FROM transaction_items ti
             JOIN items i ON ti.item_id = i.id
             WHERE ti.transaction_id = ?",
        )
        .bind(id)
        .fetch_all(&pool)
        .await
        .map_err(db_err)?;

        let receipt_items: Vec<(String, u32, f32)> = items
            .into_iter()
            .map(|it| (it.item_name, it.quantity as u32, it.unit_price as f32))
            .collect();

        let local_now = chrono::Local::now();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok((_, mut printer)) = find_printer() {
                let _ = print_receipt(
                    &mut printer,
                    receipt_items,
                    paid_amount as f32,
                    change as f32,
                    local_now,
                );
            }
        })
        .await;
    }

    Ok(CloseTransactionResponse {
        transaction,
        change_amount: change,
    })
}

#[server]
pub async fn cancel_transaction(id: Uuid) -> Result<Transaction, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let transaction = sqlx::query_as::<_, Transaction>(
        "UPDATE transactions SET status = 'cancelled', updated_at = ?
         WHERE id = ? AND status = 'open' RETURNING *",
    )
    .bind(Utc::now())
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?
    .ok_or_else(|| not_found("Transaction not found or not open"))?;
    Ok(transaction)
}

// Reports

#[server]
pub async fn fetch_sales_report(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<SalesReport, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    generate_sales_report_db(&pool, start_date, end_date).await
}

#[server]
pub async fn fetch_daily_report() -> Result<SalesReport, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let end_date = Utc::now();
    let start_date = end_date - chrono::Duration::days(1);
    generate_sales_report_db(&pool, start_date, end_date).await
}

#[server]
pub async fn fetch_monthly_report() -> Result<SalesReport, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let end_date = Utc::now();
    let start_date = end_date - chrono::Duration::days(30);
    generate_sales_report_db(&pool, start_date, end_date).await
}

// ---- Shell (SSR only) ----

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

// ---- Components ----

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/rustpos.css"/>
        <Title text="RustPOS"/>

        <Router>
            <nav class="navbar">
                <div class="nav-container">
                    <img class="sitelogo" src="/logo_site.png"/>
                    <div class="nav-links">
                        <A href="/">"Sale"</A>
                        <A href="/transactions">"Transactions"</A>
                        <A href="/items">"Items"</A>
                        <A href="/categories">"Categories"</A>
                        <A href="/reports">"Reports"</A>
                    </div>
                </div>
            </nav>

            <main class="container">
                <Routes fallback=|| "Page not found">
                    <Route path=StaticSegment("") view=SalePage/>
                    <Route path=StaticSegment("transactions") view=TransactionsPage/>
                    <Route path=StaticSegment("items") view=ItemsPage/>
                    <Route path=StaticSegment("categories") view=CategoriesPage/>
                    <Route path=StaticSegment("reports") view=ReportsPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn SalePage() -> impl IntoView {
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

    // Load initial data
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
        transaction_items
            .get()
            .iter()
            .map(|i| i.total_price)
            .sum::<f64>()
    };

    let start_transaction = move |_| {
        let name = customer_name.get();
        let set_current_transaction = set_current_transaction.clone();
        let set_transaction_items = set_transaction_items.clone();
        let set_change_amount = set_change_amount.clone();
        let set_open_transactions = set_open_transactions.clone();

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
        let set_current_transaction = set_current_transaction.clone();
        let set_transaction_items = set_transaction_items.clone();
        let set_show_open_transactions = set_show_open_transactions.clone();
        let set_customer_name = set_customer_name.clone();

        leptos::task::spawn_local(async move {
            if let Ok(details) = fetch_transaction_details(trans_id).await {
                set_current_transaction.set(Some(trans_id));
                set_transaction_items.set(details.items);
                set_customer_name
                    .set(details.transaction.customer_name.unwrap_or_default());
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
        let set_transaction_items = set_transaction_items.clone();

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
        let set_transaction_items = set_transaction_items.clone();

        if let Some(trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                if remove_item_from_transaction(trans_id, item_id)
                    .await
                    .is_ok()
                {
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
        let set_change_amount = set_change_amount.clone();
        let set_current_transaction = set_current_transaction.clone();
        let set_open_transactions = set_open_transactions.clone();
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
        let set_current_transaction = set_current_transaction.clone();
        let set_transaction_items = set_transaction_items.clone();
        let set_open_transactions = set_open_transactions.clone();
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
        let set_current_transaction = set_current_transaction.clone();
        let set_transaction_items = set_transaction_items.clone();
        let set_open_transactions = set_open_transactions.clone();

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
        <Show
            when=move || canceling_transaction.get().is_some()
            fallback=|| ()
        >
            {move || {
                canceling_transaction.get().map(|_| {
                    view! {
                        <div class="modal-overlay">
                            <div class="confirmation-modal">
                                <h3>"Confirm Delete"</h3>
                                <p>"Are you sure you want to delete this transaction?"</p>
                                <p class="warning-text">"This action cannot be undone."</p>
                                <div class="modal-actions">
                                    <button class="btn-danger" on:click=cancel_sale_handler>
                                        "Delete"
                                    </button>
                                    <button class="btn-secondary" on:click=cancel_cancel_sale>
                                        "Cancel"
                                    </button>
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
                        >
                            "All"
                        </button>
                        <For
                            each=move || categories.get()
                            key=|cat| cat.id
                            let:cat
                        >
                            {
                                let cat_id = cat.id;
                                view! {
                                    <button
                                        class=move || if selected_category.get() == Some(cat_id) { "active" } else { "" }
                                        on:click=move |_| set_selected_category.set(Some(cat_id))
                                    >
                                        {cat.name.clone()}
                                    </button>
                                }
                            }
                        </For>
                    </div>

                    <div class="items-grid">
                        <For
                            each=filtered_items
                            key=|item| item.id
                            let:item
                        >
                            {
                                let item_clone = item.clone();
                                view! {
                                    <button
                                        class="item-card"
                                        on:click=move |_| add_item(item_clone.clone())
                                        disabled=move || current_transaction.get().is_none()
                                    >
                                        <div class="item-name">{item.name.clone()}</div>
                                        <div class="item-price">{format!("{} {:.2}", CURRENCY_SYMBOL, item.price)}</div>
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
                                <button class="btn-primary" on:click=start_transaction>
                                    "New Transaction"
                                </button>

                                <Show when=move || !open_transactions.get().is_empty() fallback=|| ()>
                                    <button
                                        class="btn-secondary"
                                        on:click=move |_| set_show_open_transactions.set(!show_open_transactions.get())
                                    >
                                        {move || if show_open_transactions.get() { "Hide" } else { "Show" }}
                                        " Open Transactions ("
                                        {move || open_transactions.get().len()}
                                        ")"
                                    </button>
                                </Show>

                                <Show
                                    when=move || last_closed_transaction.get().is_some()
                                    fallback=|| ()
                                >
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
                                        <For
                                            each=move || open_transactions.get()
                                            key=|t| t.id
                                            let:trans
                                        >
                                            {
                                                let trans_id = trans.id;
                                                view! {
                                                    <div class="open-transaction-item">
                                                        <div>
                                                            <strong>{trans.customer_name.clone().unwrap_or_else(|| "Walk-in".to_string())}</strong>
                                                            <span>" - "{format!("{} {:.2}", CURRENCY_SYMBOL, trans.total)}</span>
                                                        </div>
                                                        <button
                                                            class="btn-small"
                                                            on:click=move |_| resume_transaction(trans_id)
                                                        >
                                                            "Resume"
                                                        </button>
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
                                            <td>
                                                <strong>"Customer: "</strong>
                                            </td>
                                            <td>
                                                <input
                                                    type="text"
                                                    placeholder="Walk-in"
                                                    on:input=move |ev| set_customer_name.set(event_target_value(&ev))
                                                    value=move || customer_name.get()
                                                />
                                            </td>
                                            <td class="customer-table-actions">
                                                <button class="btn-primary-small" on:click=do_update_transaction>
                                                    "Update"
                                                </button>
                                            </td>
                                        </tr>
                                    </tbody>
                                </table>
                            </div>

                            <div class="transaction-items">
                                <table class="data-table">
                                    <tbody>
                                <For
                                    each=move || transaction_items.get()
                                            key=|item| (item.id, item.quantity)
                                    let:item
                                >
                                    {
                                        let item_id = item.item_id;
                                        view! {
                                                    <tr>
                                                        <td>{item.item_name.clone()}</td>
                                                        <td>{format!("{}x", item.quantity)}</td>
                                                        <td>{format!("{} {:.2}", CURRENCY_SYMBOL, item.total_price)}</td>
                                                        <td class="data-table-actions">
                                                <button
                                                    class="btn-remove"
                                                    on:click=move |_| remove_item(item_id)
                                                >
                                                                "-"
                                                </button>
                                                        </td>
                                                    </tr>
                                        }
                                    }
                                </For>
                                    </tbody>
                                </table>
                            </div>

                            <div class="transaction-total">
                                <strong>"Total: "</strong>
                                <strong>{move || format!("{} {:.2}", CURRENCY_SYMBOL, transaction_total())}</strong>
                            </div>

                            <div class="payment-change-wrapper">
                                <div class="payment-section">
                                    <strong>"Cash: "</strong>
                                    <input
                                        type="text"
                                        class="payment-input"
                                        placeholder=""
                                        readonly
                                        value=move || payment_amount.get()
                                    />
                                </div>
                                <div class="change-section">
                                    <strong>"Change: "</strong>
                                    <input
                                        type="text"
                                        class="change-input"
                                        placeholder=""
                                        readonly
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
                                    <For
                                        each=|| vec!["7","8","9","4","5","6","1","2","3","0",".","⌫"]
                                        key=|val| val.to_string()
                                        let:val
                                    >
                                        {
                                            let val_clone = val.to_string();
                                            view! {
                                                <button
                                                    class="keypad-btn"
                                                    on:click=move |_| {
                                                        if val_clone == "⌫" {
                                                            set_payment_amount.update(|amt| { amt.pop(); });
                                                        } else {
                                                            set_payment_amount.update(|amt| amt.push_str(&val_clone));
                                                        }
                                                    }
                                                >
                                                    {val}
                                                </button>
                                            }
                                        }
                                    </For>
                                </div>
                            </div>

                            <div class="action-buttons">
                                <button class="action-button cancel" on:click=move |_| confirm_cancel_sale(current_transaction.get().unwrap_or_default())>
                                    "Cancel"
                                </button>
                                <button class="action-button pause" on:click=pause_sale>
                                    "Back"
                                </button>
                                <button class="action-button sale" on:click=checkout>
                                    "Checkout"
                                </button>
                            </div>
                            <Show
                                when=move || change_amount.get().is_some()
                                fallback=|| ()
                            >
                                <div class="change-display">
                                    <h3>
                                        "Change: "
                                        {move || format!("{} {:.2}", CURRENCY_SYMBOL, change_amount.get().unwrap())}
                                    </h3>
                                </div>
                            </Show>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
fn TransactionsPage() -> impl IntoView {
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
                <button
                    class="btn-secondary"
                    on:click=move |_| set_show_all.set(!show_all.get())
                >
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
                    <For
                        each=move || transactions.get()
                        key=|t| t.id
                        let:transaction
                    >
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

#[component]
fn ItemsPage() -> impl IntoView {
    let (items, set_items) = signal(Vec::<Item>::new());
    let (categories, set_categories) = signal(Vec::<Category>::new());
    let (editing_item, set_editing_item) = signal(Option::<Item>::None);
    let (creating_item, set_creating_item) = signal(false);
    let (deleting_item, set_deleting_item) = signal(Option::<(Uuid, String)>::None);

    // Form fields
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (price, set_price) = signal(String::new());
    let (category_id, set_category_id) = signal(String::new());
    let (sku, set_sku) = signal(String::new());
    let (in_stock, set_in_stock) = signal(true);

    let load_data = move || {
        leptos::task::spawn_local(async move {
            if let Ok(items_data) = fetch_items().await {
                set_items.set(items_data);
            }
            if let Ok(cats) = fetch_categories().await {
                set_categories.set(cats);
            }
        });
    };

    Effect::new(load_data.clone());

    let start_edit = move |item: Item| {
        set_name.set(item.name.clone());
        set_description.set(item.description.clone().unwrap_or_default());
        set_price.set(item.price.to_string());
        set_category_id.set(item.category_id.to_string());
        set_sku.set(item.sku.clone().unwrap_or_default());
        set_in_stock.set(item.in_stock);
        set_editing_item.set(Some(item));
    };

    let save_item = move |_| {
        let editing = editing_item.get();
        let creating = creating_item.get();

        if let Ok(price_val) = price.get().parse::<f64>() {
            if let Ok(cat_id) = category_id.get().parse::<Uuid>() {
                if creating {
                    let n = name.get();
                    let d = Some(description.get()).filter(|s| !s.is_empty());
                    let s = Some(sku.get()).filter(|s| !s.is_empty());
                    let stock = Some(in_stock.get());

                    leptos::task::spawn_local(async move {
                        if create_item(n, d, price_val, cat_id, s, stock)
                            .await
                            .is_ok()
                        {
                            load_data();
                            set_creating_item.set(false);
                        }
                    });
                } else if let Some(item) = editing {
                    let n = Some(name.get());
                    let d = Some(description.get()).filter(|s| !s.is_empty());
                    let s = Some(sku.get()).filter(|s| !s.is_empty());
                    let stock = Some(in_stock.get());
                    let item_id = item.id;

                    leptos::task::spawn_local(async move {
                        if update_item(
                            item_id,
                            n,
                            d,
                            Some(price_val),
                            Some(cat_id),
                            s,
                            stock,
                        )
                        .await
                        .is_ok()
                        {
                            load_data();
                            set_editing_item.set(None);
                        }
                    });
                }
            }
        }
    };

    let confirm_delete = move |id: Uuid, name: String| {
        set_deleting_item.set(Some((id, name)));
    };

    let delete_item_handler = move |_| {
        if let Some((id, _)) = deleting_item.get() {
            leptos::task::spawn_local(async move {
                if delete_item(id).await.is_ok() {
                    load_data();
                    set_deleting_item.set(None);
                }
            });
        }
    };

    let cancel_delete = move |_| {
        set_deleting_item.set(None);
    };

    let cancel_edit = move |_| {
        set_editing_item.set(None);
        set_creating_item.set(false);
        set_name.set(String::new());
        set_description.set(String::new());
        set_price.set(String::new());
        set_category_id.set(String::new());
        set_sku.set(String::new());
        set_in_stock.set(true);
    };

    let start_create = move |_| {
        set_name.set(String::new());
        set_description.set(String::new());
        set_price.set(String::new());
        set_category_id.set(if let Some(cat) = categories.get().first() {
            cat.id.to_string()
        } else {
            String::new()
        });
        set_sku.set(String::new());
        set_in_stock.set(true);
        set_creating_item.set(true);
        set_editing_item.set(None);
    };

    view! {
        <div>
            <div class="page-header">
                <h2>"Items"</h2>
                <button class="btn-primary" on:click=start_create>
                    "Add New Item"
                </button>
            </div>

            <Show
                when=move || deleting_item.get().is_some()
                fallback=|| ()
            >
                {move || {
                    deleting_item.get().map(|(_, name)| {
                        view! {
                            <div class="modal-overlay">
                                <div class="confirmation-modal">
                                    <h3>"Confirm Delete"</h3>
                                    <p>"Are you sure you want to delete \""<strong>{name}</strong>"\"?"</p>
                                    <p class="warning-text">"This action cannot be undone."</p>
                                    <div class="modal-actions">
                                        <button class="btn-danger" on:click=delete_item_handler>
                                            "Delete"
                                        </button>
                                        <button class="btn-secondary" on:click=cancel_delete>
                                            "Cancel"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                }}
            </Show>

            <Show
                when=move || editing_item.get().is_some() || creating_item.get()
                fallback=|| ()
            >
                <div class="edit-form">
                    <h3>{move || if creating_item.get() { "Create New Item" } else { "Edit Item" }}</h3>
                    <div class="form-grid">
                        <div class="form-group">
                            <label>"Name"</label>
                            <input
                                type="text"
                                value=move || name.get()
                                on:input=move |ev| set_name.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-group">
                            <label>"Price"</label>
                            <input
                                type="number"
                                step="0.01"
                                value=move || price.get()
                                on:input=move |ev| set_price.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-group">
                            <label>"Category"</label>
                            <select
                                prop:value=move || category_id.get()
                                on:change=move |ev| set_category_id.set(event_target_value(&ev))
                            >
                                <For
                                    each=move || categories.get()
                                    key=|cat| cat.id
                                    let:cat
                                >
                                    <option value={cat.id.to_string()}>
                                        {cat.name.clone()}
                                    </option>
                                </For>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>"SKU"</label>
                            <input
                                type="text"
                                value=move || sku.get()
                                on:input=move |ev| set_sku.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-group">
                            <label>"Description"</label>
                            <input
                                type="text"
                                value=move || description.get()
                                on:input=move |ev| set_description.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-group">
                            <label>
                                <input
                                    type="checkbox"
                                    checked=move || in_stock.get()
                                    on:change=move |ev| set_in_stock.set(event_target_checked(&ev))
                                />
                                " In Stock"
                            </label>
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn-success" on:click=save_item>
                            "Save"
                        </button>
                        <button class="btn-secondary" on:click=cancel_edit>
                            "Cancel"
                        </button>
                    </div>
                </div>
            </Show>

            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Name"</th>
                        <th>"Price"</th>
                        <th>"Category"</th>
                        <th>"SKU"</th>
                        <th>"In Stock"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || items.get()
                        key=|i| i.id
                        let:item
                    >
                        {
                            let item_clone = item.clone();
                            let item_id = item.id;
                            let item_name = item.name.clone();
                            let category_name = categories.get()
                                .iter()
                                .find(|c| c.id == item.category_id)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| "Unknown".to_string());

                            view! {
                                <tr>
                                    <td>{item.name.clone()}</td>
                                    <td>{format!("{} {:.2}", CURRENCY_SYMBOL, item.price)}</td>
                                    <td>{category_name}</td>
                                    <td>{item.sku.clone().unwrap_or_else(|| "-".to_string())}</td>
                                    <td>{if item.in_stock { "✓" } else { "✗" }}</td>
                                    <td class="data-table-actions">
                                        <button
                                            class="btn-small"
                                            on:click=move |_| start_edit(item_clone.clone())
                                        >
                                            "Edit"
                                        </button>
                                        <button
                                            class="btn-small btn-danger"
                                            on:click=move |_| confirm_delete(item_id, item_name.clone())
                                        >
                                            "Delete"
                                        </button>
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

#[component]
fn CategoriesPage() -> impl IntoView {
    let (categories, set_categories) = signal(Vec::<Category>::new());
    let (editing_category, set_editing_category) = signal(Option::<Category>::None);
    let (creating_category, set_creating_category) = signal(false);
    let (deleting_category, set_deleting_category) =
        signal(Option::<(Uuid, String)>::None);

    // Form fields
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());

    let load_categories = move || {
        leptos::task::spawn_local(async move {
            if let Ok(cats) = fetch_categories().await {
                set_categories.set(cats);
            }
        });
    };

    Effect::new(load_categories.clone());

    let start_edit = move |category: Category| {
        set_name.set(category.name.clone());
        set_description.set(category.description.clone().unwrap_or_default());
        set_editing_category.set(Some(category));
    };

    let save_category = move |_| {
        let editing = editing_category.get();
        let creating = creating_category.get();

        if creating {
            let n = name.get();
            let d = Some(description.get()).filter(|s| !s.is_empty());

            leptos::task::spawn_local(async move {
                if create_category(n, d).await.is_ok() {
                    load_categories();
                    set_creating_category.set(false);
                }
            });
        } else if let Some(category) = editing {
            let n = Some(name.get());
            let d = Some(description.get()).filter(|s| !s.is_empty());
            let cat_id = category.id;

            leptos::task::spawn_local(async move {
                if update_category(cat_id, n, d).await.is_ok() {
                    load_categories();
                    set_editing_category.set(None);
                }
            });
        }
    };

    let confirm_delete = move |id: Uuid, name: String| {
        set_deleting_category.set(Some((id, name)));
    };

    let delete_category_handler = move |_| {
        if let Some((id, _)) = deleting_category.get() {
            leptos::task::spawn_local(async move {
                if delete_category(id).await.is_ok() {
                    load_categories();
                    set_deleting_category.set(None);
                }
            });
        }
    };

    let cancel_delete = move |_| {
        set_deleting_category.set(None);
    };

    let cancel_edit = move |_| {
        set_editing_category.set(None);
        set_creating_category.set(false);
        set_name.set(String::new());
        set_description.set(String::new());
    };

    let start_create = move |_| {
        set_name.set(String::new());
        set_description.set(String::new());
        set_creating_category.set(true);
        set_editing_category.set(None);
    };

    view! {
        <div>
            <div class="page-header">
                <h2>"Categories"</h2>
                <button class="btn-primary" on:click=start_create>
                    "Add New Category"
                </button>
            </div>

            <Show
                when=move || deleting_category.get().is_some()
                fallback=|| ()
            >
                {move || {
                    deleting_category.get().map(|(_, name)| {
                        view! {
                            <div class="modal-overlay">
                                <div class="confirmation-modal">
                                    <h3>"Confirm Delete"</h3>
                                    <p>"Are you sure you want to delete the category \""<strong>{name}</strong>"\"?"</p>
                                    <p class="warning-text">"Warning: This will NOT delete items in this category, but they may become harder to find."</p>
                                    <div class="modal-actions">
                                        <button class="btn-danger" on:click=delete_category_handler>
                                            "Delete"
                                        </button>
                                        <button class="btn-secondary" on:click=cancel_delete>
                                            "Cancel"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                }}
            </Show>

            <Show
                when=move || editing_category.get().is_some() || creating_category.get()
                fallback=|| ()
            >
                <div class="edit-form">
                    <h3>{move || if creating_category.get() { "Create New Category" } else { "Edit Category" }}</h3>
                    <div class="form-grid">
                        <div class="form-group">
                            <label>"Name"</label>
                            <input
                                type="text"
                                value=move || name.get()
                                on:input=move |ev| set_name.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-group">
                            <label>"Description"</label>
                            <input
                                type="text"
                                value=move || description.get()
                                on:input=move |ev| set_description.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn-success" on:click=save_category>
                            "Save"
                        </button>
                        <button class="btn-secondary" on:click=cancel_edit>
                            "Cancel"
                        </button>
                    </div>
                </div>
            </Show>

            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Name"</th>
                        <th>"Description"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || categories.get()
                        key=|c| (c.id, c.description.clone(), c.name.clone())
                        let:category
                    >
                        {
                            let category_clone = category.clone();
                            let category_id = category.id;
                            let category_name = category.name.clone();

                            view! {
                                <tr>
                                    <td>{category.name.clone()}</td>
                                    <td>{category.description.clone().unwrap_or_else(|| "-".to_string())}</td>
                                    <td class="data-table-actions">
                                        <button
                                            class="btn-small"
                                            on:click=move |_| start_edit(category_clone.clone())
                                        >
                                            "Edit"
                                        </button>
                                        <button
                                            class="btn-small btn-danger"
                                            on:click=move |_| confirm_delete(category_id, category_name.clone())
                                        >
                                            "Delete"
                                        </button>
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

#[component]
fn ReportsPage() -> impl IntoView {
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
                        let start_dt = start
                            .and_hms_opt(0, 0, 0)
                            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                        let end_dt = end
                            .and_hms_opt(23, 59, 59)
                            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));

                        if let (Some(start_dt), Some(end_dt)) = (start_dt, end_dt) {
                            fetch_sales_report(start_dt, end_dt)
                                .await
                                .map_err(|e| e.to_string())
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
                    set_report.set(Some(report_data));
                    set_error.set(None);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_report.set(None);
                }
            }
            set_loading.set(false);
        });
    };

    Effect::new(move || {
        load_report("daily".to_string());
    });

    view! {
        <div class="reports-page">
            <h2>"Sales Reports"</h2>

            <div class="report-controls">
                <div class="report-type-selector">
                    <button
                        class=move || if report_type.get() == "daily" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| {
                            set_report_type.set("daily".to_string());
                            load_report("daily".to_string());
                        }
                    >
                        "Today"
                    </button>
                    <button
                        class=move || if report_type.get() == "monthly" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| {
                            set_report_type.set("monthly".to_string());
                            load_report("monthly".to_string());
                        }
                    >
                        "Last 30 Days"
                    </button>
                    <button
                        class=move || if report_type.get() == "custom" { "btn-primary" } else { "btn-secondary" }
                        on:click=move |_| set_report_type.set("custom".to_string())
                    >
                        "Custom Range"
                    </button>
                </div>

                <Show when=move || report_type.get() == "custom" fallback=|| ()>
                    <div class="date-range-selector">
                        <div class="form-group">
                            <label>"Start Date"</label>
                            <input
                                type="date"
                                value=move || start_date.get()
                                on:input=move |ev| set_start_date.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-group">
                            <label>"End Date"</label>
                            <input
                                type="date"
                                value=move || end_date.get()
                                on:input=move |ev| set_end_date.set(event_target_value(&ev))
                            />
                        </div>
                        <button
                            class="btn-primary"
                            on:click=move |_| load_report("custom".to_string())
                        >
                            "Generate Report"
                        </button>
                    </div>
                </Show>
            </div>

            <Show when=move || loading.get() fallback=|| ()>
                <div class="loading">"Loading report..."</div>
            </Show>

            <Show when=move || error.get().is_some() fallback=|| ()>
                <div class="error-message">
                    "Error: "{move || error.get().unwrap_or_default()}
                </div>
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
                                    <div class="summary-card">
                                        <h4>"Total Revenue"</h4>
                                        <div class="summary-value">{format!("{} {:.2}", CURRENCY_SYMBOL, report_data.summary.total_revenue)}</div>
                                    </div>
                                    <div class="summary-card">
                                        <h4>"Items Sold"</h4>
                                        <div class="summary-value">{report_data.summary.total_items_sold.to_string()}</div>
                                    </div>
                                    <div class="summary-card">
                                        <h4>"Transactions"</h4>
                                        <div class="summary-value">{report_data.summary.total_transactions.to_string()}</div>
                                    </div>
                                    <div class="summary-card">
                                        <h4>"Avg Transaction"</h4>
                                        <div class="summary-value">{format!("{} {:.2}", CURRENCY_SYMBOL, report_data.summary.average_transaction_value)}</div>
                                    </div>
                                </div>

                                <div class="report-highlights">
                                    {report_data.summary.top_selling_item.as_ref().map(|item| {
                                        view! {
                                            <div class="highlight">
                                                <strong>"Top Selling Item: "</strong>
                                                {item.clone()}
                                            </div>
                                        }
                                    })}
                                    {report_data.summary.top_revenue_item.as_ref().map(|item| {
                                        view! {
                                            <div class="highlight">
                                                <strong>"Top Revenue Item: "</strong>
                                                {item.clone()}
                                            </div>
                                        }
                                    })}
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
                                            <thead>
                                                <tr>
                                                    <th>"Item"</th>
                                                    <th>"Category"</th>
                                                    <th>"Quantity Sold"</th>
                                                    <th>"Revenue"</th>
                                                    <th>"Avg Price"</th>
                                                    <th>"Transactions"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                <For
                                                    each=move || items.clone()
                                                    key=|item| item.item_id
                                                    let:item
                                                >
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
