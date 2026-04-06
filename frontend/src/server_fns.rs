use chrono::{DateTime, Utc};
use leptos::prelude::*;
use uuid::Uuid;

use crate::models::*;

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

// ---- Category Server Functions ----

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

// ---- Item Server Functions ----

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

    if let Some(n) = name { item.name = n; }
    if let Some(d) = description { item.description = Some(d); }
    if let Some(p) = price { item.price = p; }
    if let Some(c) = category_id { item.category_id = c; }
    if let Some(s) = sku { item.sku = Some(s); }
    if let Some(s) = in_stock { item.in_stock = s; }
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

// ---- Item Image Server Functions ----

#[server]
pub async fn upload_item_image(
    item_id: Uuid,
    image_data: String,
) -> Result<String, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    // Validate item exists
    sqlx::query("SELECT id FROM items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Item not found"))?;

    // image_data is a base64 data URL like "data:image/png;base64,..."
    // Extract the raw base64 and mime type
    let (mime, b64) = image_data
        .strip_prefix("data:")
        .and_then(|s| s.split_once(','))
        .ok_or_else(|| not_found("Invalid image data"))?;

    let ext = if mime.starts_with("image/png") {
        "png"
    } else if mime.starts_with("image/jpeg") {
        "jpg"
    } else if mime.starts_with("image/webp") {
        "webp"
    } else {
        "png"
    };

    use std::io::Write;
    let bytes = base64_decode(b64).map_err(|e| db_err(e))?;

    let dir = "data/item_images";
    std::fs::create_dir_all(dir).map_err(|e| db_err(e))?;

    let filename = format!("{}.{}", item_id, ext);
    let filepath = format!("{}/{}", dir, filename);

    let mut file = std::fs::File::create(&filepath).map_err(|e| db_err(e))?;
    file.write_all(&bytes).map_err(|e| db_err(e))?;

    let url_path = format!("/item_images/{}", filename);

    sqlx::query("UPDATE items SET image_path = ?, updated_at = ? WHERE id = ?")
        .bind(&url_path)
        .bind(Utc::now())
        .bind(item_id)
        .execute(&pool)
        .await
        .map_err(db_err)?;

    Ok(url_path)
}

#[server]
pub async fn remove_item_image(item_id: Uuid) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    let image_path: Option<String> =
        sqlx::query_scalar("SELECT image_path FROM items WHERE id = ?")
            .bind(item_id)
            .fetch_optional(&pool)
            .await
            .map_err(db_err)?
            .flatten();

    if let Some(ref url_path) = image_path {
        let fs_path = format!("data{}", url_path);
        let _ = std::fs::remove_file(&fs_path);
    }

    sqlx::query("UPDATE items SET image_path = NULL, updated_at = ? WHERE id = ?")
        .bind(Utc::now())
        .bind(item_id)
        .execute(&pool)
        .await
        .map_err(db_err)?;

    Ok(())
}

#[cfg(feature = "ssr")]
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    // Simple base64 decoder — no extra dependency needed
    use std::collections::HashMap;
    let table: HashMap<u8, u8> = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter()
        .enumerate()
        .map(|(i, &c)| (c, i as u8))
        .collect();

    let input: Vec<u8> = input.bytes().filter(|&b| b != b'\n' && b != b'\r' && b != b' ').collect();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);

    for chunk in input.chunks(4) {
        let mut buf = [0u8; 4];
        let mut len = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                break;
            }
            buf[i] = *table.get(&b).ok_or_else(|| format!("Invalid base64 char: {}", b as char))?;
            len = i + 1;
        }
        if len >= 2 { out.push((buf[0] << 2) | (buf[1] >> 4)); }
        if len >= 3 { out.push((buf[1] << 4) | (buf[2] >> 2)); }
        if len >= 4 { out.push((buf[2] << 6) | buf[3]); }
    }
    Ok(out)
}

// ---- Transaction Server Functions ----

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

    sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'",
    )
    .bind(transaction_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?
    .ok_or_else(|| not_found("Transaction not found or not open"))?;

    let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Item not found"))?;

    if !item.in_stock {
        return Err(not_found("Item is out of stock"));
    }

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
        sqlx::query("DELETE FROM transaction_items WHERE transaction_id = ? AND item_id = ?")
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
            sqlx::query("DELETE FROM transaction_items WHERE transaction_id = ? AND item_id = ?")
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

// ---- Report Server Functions ----

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
