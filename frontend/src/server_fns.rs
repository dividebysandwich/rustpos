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
fn hash_pin(pin: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(pin.as_bytes());
    hasher.finalize().iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
        s
    })
}

#[cfg(feature = "ssr")]
fn extract_session_token(headers: &axum::http::HeaderMap) -> Option<String> {
    let cookie_str = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    cookie_str
        .split(';')
        .filter_map(|c| c.trim().strip_prefix("rustpos_session=").map(|s| s.to_string()))
        .next()
}

#[cfg(feature = "ssr")]
async fn get_authenticated_user(pool: &sqlx::SqlitePool) -> Result<Option<UserAccount>, ServerFnError> {
    let Ok(headers) = leptos_axum::extract::<axum::http::HeaderMap>().await else {
        return Ok(None);
    };
    let Some(token) = extract_session_token(&headers) else {
        return Ok(None);
    };
    let user = sqlx::query_as::<_, UserAccount>(
        "SELECT u.* FROM users u JOIN sessions s ON u.id = s.user_id WHERE s.token = ? AND s.expires_at > ?",
    )
    .bind(&token)
    .bind(Utc::now())
    .fetch_optional(pool)
    .await
    .map_err(db_err)?;
    Ok(user)
}

#[cfg(feature = "ssr")]
async fn require_admin(pool: &sqlx::SqlitePool) -> Result<UserAccount, ServerFnError> {
    let user = get_authenticated_user(pool).await?
        .ok_or_else(|| not_found("Not authenticated"))?;
    if user.role != "admin" {
        return Err(not_found("Admin access required"));
    }
    Ok(user)
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

/// Builds the SQL fragment that restricts a query to a customer group.
///
/// `alias` is the table alias used for `transactions` in the surrounding query
/// (e.g. `"t"` or `"transactions"`); it is a hard-coded identifier, never user
/// input. A `Group` id is a `Uuid` whose canonical hex form has no SQL-special
/// characters, so inlining it is safe and avoids variadic binding.
#[cfg(feature = "ssr")]
fn group_filter_clause(filter: &GroupFilter, alias: &str) -> String {
    match filter {
        GroupFilter::All => String::new(),
        GroupFilter::Regular => format!(" AND {alias}.customer_group_id IS NULL"),
        GroupFilter::Group(id) => format!(" AND {alias}.customer_group_id = '{id}'"),
    }
}

#[cfg(feature = "ssr")]
async fn generate_sales_report_db(
    pool: &sqlx::SqlitePool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    filter: &GroupFilter,
) -> Result<SalesReport, ServerFnError> {
    if end_date <= start_date {
        return Err(not_found("End date must be after start date"));
    }

    let items = sqlx::query_as::<_, ItemSalesReport>(&format!(
        "SELECT i.id as item_id, i.name as item_name, c.name as category_name,
         SUM(ti.quantity) as quantity_sold, SUM(ti.total_price) as total_revenue,
         AVG(ti.unit_price) as average_price, COUNT(DISTINCT ti.transaction_id) as transaction_count
         FROM transaction_items ti
         JOIN items i ON ti.item_id = i.id
         JOIN categories c ON i.category_id = c.id
         JOIN transactions t ON ti.transaction_id = t.id
         WHERE t.status = 'closed' AND t.closed_at >= ? AND t.closed_at < ?{}
         GROUP BY i.id, i.name, c.name ORDER BY total_revenue DESC",
        group_filter_clause(filter, "t"),
    ))
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total_revenue: f64 = items.iter().map(|i| i.total_revenue).sum();
    let total_items_sold: i64 = items.iter().map(|i| i.quantity_sold).sum();

    let transaction_count = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(DISTINCT id) FROM transactions
         WHERE status = 'closed' AND closed_at >= ? AND closed_at < ?{}",
        group_filter_clause(filter, "transactions"),
    ))
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
    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY sort_order, name")
        .fetch_all(&pool)
        .await
        .map_err(db_err)?;
    Ok(categories)
}

#[server]
pub async fn create_category(
    name: String,
    description: Option<String>,
    main_course: Option<bool>,
) -> Result<Category, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let id = Uuid::new_v4();
    let now = Utc::now();
    let main_course = main_course.unwrap_or(false);
    // Append new categories at the end of the user-defined order.
    let next_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM categories")
            .fetch_one(&pool)
            .await
            .map_err(db_err)?;
    let category = sqlx::query_as::<_, Category>(
        "INSERT INTO categories (id, name, description, main_course, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(&name)
    .bind(&description)
    .bind(main_course)
    .bind(next_order)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(category)
}

/// Moves a category one position up or down in the user-defined order by
/// swapping its `sort_order` with its nearest neighbour. No-op at the ends.
#[server]
pub async fn move_category(id: Uuid, up: bool) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    let current = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Category not found"))?;

    // Find the adjacent category in (sort_order, name) order, using the same
    // tie-break as fetch_categories so the swap matches the displayed order.
    let neighbor = if up {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM categories
             WHERE sort_order < ? OR (sort_order = ? AND name < ?)
             ORDER BY sort_order DESC, name DESC LIMIT 1",
        )
    } else {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM categories
             WHERE sort_order > ? OR (sort_order = ? AND name > ?)
             ORDER BY sort_order ASC, name ASC LIMIT 1",
        )
    }
    .bind(current.sort_order)
    .bind(current.sort_order)
    .bind(&current.name)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?;

    let Some(neighbor) = neighbor else {
        return Ok(()); // already at the top/bottom
    };

    // Swap the two order values. If they happen to be equal (legacy data),
    // nudge them apart so the move still takes effect.
    let (cur_new, nb_new) = if current.sort_order == neighbor.sort_order {
        if up {
            (neighbor.sort_order - 1, neighbor.sort_order)
        } else {
            (neighbor.sort_order + 1, neighbor.sort_order)
        }
    } else {
        (neighbor.sort_order, current.sort_order)
    };

    sqlx::query("UPDATE categories SET sort_order = ? WHERE id = ?")
        .bind(cur_new)
        .bind(current.id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    sqlx::query("UPDATE categories SET sort_order = ? WHERE id = ?")
        .bind(nb_new)
        .bind(neighbor.id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    Ok(())
}

#[server]
pub async fn update_category(
    id: Uuid,
    name: Option<String>,
    description: Option<String>,
    main_course: Option<bool>,
) -> Result<Category, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let mut category = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Category not found"))?;

    if let Some(n) = name { category.name = n; }
    if let Some(d) = description { category.description = Some(d); }
    if let Some(mc) = main_course { category.main_course = mc; }
    category.updated_at = Utc::now();

    let updated = sqlx::query_as::<_, Category>(
        "UPDATE categories SET name = ?, description = ?, main_course = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&category.name)
    .bind(&category.description)
    .bind(category.main_course)
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

// ---- Customer Group Server Functions ----

/// Lists all customer groups. Available to any signed-in role so cashiers can
/// pick a group when closing a sale.
#[server]
pub async fn fetch_customer_groups() -> Result<Vec<CustomerGroup>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let groups = sqlx::query_as::<_, CustomerGroup>("SELECT * FROM customer_groups ORDER BY name")
        .fetch_all(&pool)
        .await
        .map_err(db_err)?;
    Ok(groups)
}

#[server]
pub async fn create_customer_group(name: String) -> Result<CustomerGroup, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(not_found("Group name must not be empty"));
    }
    let id = Uuid::new_v4();
    let now = Utc::now();
    let group = sqlx::query_as::<_, CustomerGroup>(
        "INSERT INTO customer_groups (id, name, created_at, updated_at)
         VALUES (?, ?, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(&name)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;
    Ok(group)
}

#[server]
pub async fn update_customer_group(id: Uuid, name: String) -> Result<CustomerGroup, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(not_found("Group name must not be empty"));
    }
    let group = sqlx::query_as::<_, CustomerGroup>(
        "UPDATE customer_groups SET name = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&name)
    .bind(Utc::now())
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(db_err)?
    .ok_or_else(|| not_found("Customer group not found"))?;
    Ok(group)
}

/// Deletes a customer group. Its sales are transferred back to "regular
/// customers" (their `customer_group_id` is cleared) so no statistics are lost.
#[server]
pub async fn delete_customer_group(id: Uuid) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;

    sqlx::query("UPDATE transactions SET customer_group_id = NULL WHERE customer_group_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(db_err)?;

    let result = sqlx::query("DELETE FROM customer_groups WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(not_found("Customer group not found"));
    }
    Ok(())
}

/// Generates a printable PDF menu sheet and returns it base64-encoded.
///
/// The sheet shows the logo and `title`, then the available items of every
/// "main course" category with images and prices, followed by the remaining
/// categories as image-less sections. Only items that are in stock are listed,
/// and empty categories are skipped.
#[server]
pub async fn generate_menu_pdf(title: String) -> Result<String, ServerFnError> {
    use base64::Engine;
    use crate::menu_pdf::{build_menu_pdf, MenuItem, MenuSection};

    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;

    // User-defined order; the PDF builder still groups main courses first,
    // preserving this order within each group.
    let categories = sqlx::query_as::<_, Category>(
        "SELECT * FROM categories ORDER BY sort_order, name",
    )
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    let currency: String = sqlx::query_scalar("SELECT value FROM config WHERE key = 'currency'")
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let mut sections: Vec<MenuSection> = Vec::new();
    for category in &categories {
        let items = sqlx::query_as::<_, Item>(
            "SELECT * FROM items WHERE category_id = ? AND in_stock = 1 ORDER BY name",
        )
        .bind(category.id)
        .fetch_all(&pool)
        .await
        .map_err(db_err)?;

        if items.is_empty() {
            continue;
        }

        sections.push(MenuSection {
            name: category.name.clone(),
            main_course: category.main_course,
            items: items
                .into_iter()
                .map(|i| MenuItem {
                    name: i.name,
                    price: i.price,
                    description: i.description,
                    // `image_path` is a web URL ("/item_images/..."); map it to
                    // the on-disk path served from the data directory.
                    image_path: i.image_path.map(|p| format!("data{}", p)),
                })
                .collect(),
        });
    }

    // Prefer the site logo (bundled into the `site` root as /logo_site.png),
    // falling back to the receipt logo if it isn't present.
    let logo_path = if std::path::Path::new("site/logo_site.png").exists() {
        "site/logo_site.png"
    } else {
        "data/logo_receipt.png"
    };

    let pdf_bytes = tokio::task::spawn_blocking(move || {
        build_menu_pdf(&title, &currency, logo_path, &sections)
    })
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .map_err(ServerFnError::new)?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&pdf_bytes))
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
    stock_quantity: Option<i32>,
    kitchen_item: Option<bool>,
) -> Result<Item, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let id = Uuid::new_v4();
    let now = Utc::now();
    let in_stock = in_stock.unwrap_or(true);
    let kitchen_item = kitchen_item.unwrap_or(false);
    let item = sqlx::query_as::<_, Item>(
        "INSERT INTO items (id, name, description, price, category_id, sku, in_stock, stock_quantity, kitchen_item, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(&name)
    .bind(&description)
    .bind(price)
    .bind(category_id)
    .bind(&sku)
    .bind(in_stock)
    .bind(stock_quantity)
    .bind(kitchen_item)
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
    stock_quantity: Option<i32>,
    track_stock: Option<bool>,
    kitchen_item: Option<bool>,
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
    if let Some(k) = kitchen_item { item.kitchen_item = k; }
    // track_stock=Some(false) means "endless" -> set stock_quantity to None
    if let Some(track) = track_stock {
        if track {
            if let Some(qty) = stock_quantity { item.stock_quantity = Some(qty); }
        } else {
            item.stock_quantity = None;
        }
    } else if let Some(qty) = stock_quantity {
        item.stock_quantity = Some(qty);
    }
    item.updated_at = Utc::now();

    let updated = sqlx::query_as::<_, Item>(
        "UPDATE items SET name = ?, description = ?, price = ?, category_id = ?,
         sku = ?, in_stock = ?, stock_quantity = ?, kitchen_item = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&item.name)
    .bind(&item.description)
    .bind(item.price)
    .bind(item.category_id)
    .bind(&item.sku)
    .bind(item.in_stock)
    .bind(item.stock_quantity)
    .bind(item.kitchen_item)
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

    sqlx::query("SELECT id FROM items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Item not found"))?;

    let (mime, b64) = image_data
        .strip_prefix("data:")
        .and_then(|s| s.split_once(','))
        .ok_or_else(|| not_found("Invalid image data"))?;

    let ext = if mime.starts_with("image/png") { "png" }
        else if mime.starts_with("image/jpeg") { "jpg" }
        else if mime.starts_with("image/webp") { "webp" }
        else { "png" };

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
            if b == b'=' { break; }
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

    // Notify other sale clients about the new open transaction
    if let Some(sb) = use_context::<crate::SaleBroadcast>() {
        let _ = sb.0.send(format!("update:{}", id));
    }

    Ok(transaction)
}

#[server]
pub async fn update_transaction_details(
    id: Uuid,
    customer_name: Option<String>,
    customer_group_id: Option<Uuid>,
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
        "UPDATE transactions SET customer_name = ?, customer_group_id = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&customer_name)
    .bind(customer_group_id)
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

    // Check stock quantity if tracked
    if let Some(stock_qty) = item.stock_quantity {
        let existing_in_transaction = sqlx::query_scalar::<_, i32>(
            "SELECT quantity FROM transaction_items WHERE transaction_id = ? AND item_id = ?",
        )
        .bind(transaction_id)
        .bind(item_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .unwrap_or(0);

        if existing_in_transaction + quantity > stock_qty {
            return Err(not_found(&format!("Only {} left in stock", stock_qty - existing_in_transaction)));
        }
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

    // Notify customer display
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<String>>() {
        let _ = tx.send(format!("update:{}", transaction_id));
    }
    // Notify other sale clients
    if let Some(sb) = use_context::<crate::SaleBroadcast>() {
        let _ = sb.0.send(format!("update:{}", transaction_id));
    }

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

    // Notify customer display
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<String>>() {
        let _ = tx.send(format!("update:{}", transaction_id));
    }
    // Notify other sale clients
    if let Some(sb) = use_context::<crate::SaleBroadcast>() {
        let _ = sb.0.send(format!("update:{}", transaction_id));
    }

    Ok(())
}

#[server]
pub async fn close_transaction(
    id: Uuid,
    paid_amount: f64,
) -> Result<CloseTransactionResponse, ServerFnError> {
    use crate::printer::{find_printer, open_cash_drawer, print_receipt};

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

    // Decrement stock quantities for tracked items
    let trans_items = sqlx::query_as::<_, TransactionItemDetail>(
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

    for ti in &trans_items {
        // Decrement stock_quantity for tracked items
        sqlx::query(
            "UPDATE items SET stock_quantity = stock_quantity - ?
             WHERE id = ? AND stock_quantity IS NOT NULL",
        )
        .bind(ti.quantity)
        .bind(ti.item_id)
        .execute(&pool)
        .await
        .map_err(db_err)?;

        // Auto-mark out of stock if stock_quantity reaches 0
        sqlx::query(
            "UPDATE items SET in_stock = 0 WHERE id = ? AND stock_quantity IS NOT NULL AND stock_quantity <= 0",
        )
        .bind(ti.item_id)
        .execute(&pool)
        .await
        .map_err(db_err)?;

        // Create kitchen order items for kitchen items
        let is_kitchen: bool = sqlx::query_scalar("SELECT kitchen_item FROM items WHERE id = ?")
            .bind(ti.item_id)
            .fetch_one(&pool)
            .await
            .map_err(db_err)?;

        if is_kitchen {
            let ko_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO kitchen_order_items (id, transaction_id, transaction_item_id, item_id, item_name, quantity, customer_name, completed, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)",
            )
            .bind(ko_id)
            .bind(id)
            .bind(ti.id)
            .bind(ti.item_id)
            .bind(&ti.item_name)
            .bind(ti.quantity)
            .bind(&transaction.customer_name)
            .bind(now)
            .execute(&pool)
            .await
            .map_err(db_err)?;
        }
    }

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
        let receipt_items: Vec<(String, u32, f32)> = trans_items
            .into_iter()
            .map(|it| (it.item_name, it.quantity as u32, it.unit_price as f32))
            .collect();

        let local_now = chrono::Local::now();

        // Send to remote printer clients via WebSocket
        if let Some(printer_tx) = use_context::<tokio::sync::broadcast::Sender<
            rustpos_common::protocol::PrintReceiptJob,
        >>() {
            let job = rustpos_common::protocol::PrintReceiptJob {
                items: receipt_items.clone(),
                paid_amount: paid_amount as f32,
                change: change as f32,
                datetime: local_now.format("%Y-%m-%d %H:%M:%S").to_string(),
            };
            let _ = printer_tx.send(job);
        }

        // Local print (unless disabled in settings — e.g. printing handled by a
        // remote client). Even when local printing is disabled, we still open the
        // cash drawer if a printer is physically connected, so cash sales work
        // without printing a local receipt.
        let local_printing_disabled = read_disable_local_printing(&pool).await;
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok((_, mut printer)) = find_printer() {
                if local_printing_disabled {
                    let _ = open_cash_drawer(&mut printer);
                } else {
                    let _ = print_receipt(
                        &mut printer,
                        receipt_items,
                        paid_amount as f32,
                        change as f32,
                        local_now,
                        Some("data/logo_receipt.png"),
                    );
                }
            }
        })
        .await;
    }

    // Notify kitchen displays via WebSocket
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<()>>() {
        let _ = tx.send(());
    }

    // Notify customer display — transaction closed, keep showing briefly
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<String>>() {
        let _ = tx.send(format!("closed:{}", id));
    }
    // Notify other sale clients
    if let Some(sb) = use_context::<crate::SaleBroadcast>() {
        let _ = sb.0.send(format!("closed:{}", id));
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

    // Notify customer display — clear immediately
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<String>>() {
        let _ = tx.send("clear".to_string());
    }
    // Notify other sale clients
    if let Some(sb) = use_context::<crate::SaleBroadcast>() {
        let _ = sb.0.send(format!("cancelled:{}", id));
    }

    Ok(transaction)
}

/// Called by the sale page to tell the customer display which transaction is active.
#[server]
pub async fn set_display_transaction(id: Option<Uuid>) -> Result<(), ServerFnError> {
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<String>>() {
        let msg = match id {
            Some(id) => format!("update:{}", id),
            None => "clear".to_string(),
        };
        let _ = tx.send(msg);
    }
    Ok(())
}

// ---- Report Server Functions ----

#[server]
pub async fn fetch_sales_report(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    filter: GroupFilter,
) -> Result<SalesReport, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    generate_sales_report_db(&pool, start_date, end_date, &filter).await
}

#[server]
pub async fn fetch_daily_report(filter: GroupFilter) -> Result<SalesReport, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let end_date = Utc::now();
    let start_date = end_date - chrono::Duration::days(1);
    generate_sales_report_db(&pool, start_date, end_date, &filter).await
}

#[server]
pub async fn fetch_monthly_report(filter: GroupFilter) -> Result<SalesReport, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let end_date = Utc::now();
    let start_date = end_date - chrono::Duration::days(30);
    generate_sales_report_db(&pool, start_date, end_date, &filter).await
}

#[server]
pub async fn fetch_item_sales_timeseries(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    top_n: i64,
    filter: GroupFilter,
) -> Result<ItemSalesTimeseries, ServerFnError> {
    use chrono::{Datelike, TimeZone, Timelike};

    if end_date <= start_date {
        return Err(not_found("End date must be after start date"));
    }
    let pool = expect_context::<sqlx::SqlitePool>();

    // Determine bucket size: hourly for short periods, daily otherwise.
    let duration = end_date.signed_duration_since(start_date);
    let use_hourly = duration <= chrono::Duration::hours(48);

    // Top N items by quantity within the period.
    #[derive(sqlx::FromRow)]
    struct TopRow {
        item_id: Uuid,
        item_name: String,
    }
    let top_items = sqlx::query_as::<_, TopRow>(&format!(
        "SELECT i.id as item_id, i.name as item_name
         FROM transaction_items ti
         JOIN items i ON ti.item_id = i.id
         JOIN transactions t ON ti.transaction_id = t.id
         WHERE t.status = 'closed' AND t.closed_at >= ? AND t.closed_at < ?{}
         GROUP BY i.id, i.name
         ORDER BY SUM(ti.quantity) DESC
         LIMIT ?",
        group_filter_clause(&filter, "t"),
    ))
    .bind(start_date)
    .bind(end_date)
    .bind(top_n)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    let item_ids: Vec<Uuid> = top_items.iter().map(|r| r.item_id).collect();
    let item_names: Vec<String> = top_items.iter().map(|r| r.item_name.clone()).collect();
    let item_index: std::collections::HashMap<Uuid, usize> = item_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();

    // Pre-compute buckets covering the full requested range so empty periods still appear.
    let mut buckets: Vec<TimeseriesBucket> = Vec::new();
    let n = item_ids.len();
    if use_hourly {
        let mut t = chrono::Utc
            .with_ymd_and_hms(
                start_date.year(),
                start_date.month(),
                start_date.day(),
                start_date.hour(),
                0,
                0,
            )
            .single()
            .unwrap_or(start_date);
        while t < end_date {
            buckets.push(TimeseriesBucket {
                bucket_start: t,
                label: t.format("%H:%M").to_string(),
                quantities: vec![0i64; n],
            });
            t = t + chrono::Duration::hours(1);
        }
    } else {
        let mut d = start_date.date_naive();
        let end_d = end_date.date_naive();
        while d <= end_d {
            let ts = d
                .and_hms_opt(0, 0, 0)
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(start_date);
            buckets.push(TimeseriesBucket {
                bucket_start: ts,
                label: d.format("%m-%d").to_string(),
                quantities: vec![0i64; n],
            });
            d = d.succ_opt().unwrap_or(d);
        }
    }

    if n == 0 || buckets.is_empty() {
        return Ok(ItemSalesTimeseries {
            item_ids,
            item_names,
            buckets,
            bucket_unit: if use_hourly { "hour".into() } else { "day".into() },
        });
    }

    // Pull raw rows for the top items and aggregate.
    let placeholders = vec!["?"; item_ids.len()].join(",");
    let sql = format!(
        "SELECT ti.item_id as item_id, ti.quantity as quantity, t.closed_at as closed_at
         FROM transaction_items ti
         JOIN transactions t ON ti.transaction_id = t.id
         WHERE t.status = 'closed' AND t.closed_at >= ? AND t.closed_at < ?{}
           AND ti.item_id IN ({})",
        group_filter_clause(&filter, "t"),
        placeholders
    );
    let mut q = sqlx::query_as::<_, (Uuid, i64, DateTime<Utc>)>(&sql)
        .bind(start_date)
        .bind(end_date);
    for id in &item_ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(&pool).await.map_err(db_err)?;

    for (item_id, qty, closed_at) in rows {
        let Some(&idx) = item_index.get(&item_id) else { continue };
        // Find bucket index by linear scan with simple math
        let bucket_idx = if use_hourly {
            let diff = closed_at.signed_duration_since(buckets[0].bucket_start);
            let h = diff.num_hours();
            if h < 0 { continue; }
            h as usize
        } else {
            let diff = closed_at.date_naive().signed_duration_since(buckets[0].bucket_start.date_naive());
            let d = diff.num_days();
            if d < 0 { continue; }
            d as usize
        };
        if let Some(b) = buckets.get_mut(bucket_idx) {
            b.quantities[idx] += qty;
        }
    }

    Ok(ItemSalesTimeseries {
        item_ids,
        item_names,
        buckets,
        bucket_unit: if use_hourly { "hour".into() } else { "day".into() },
    })
}

#[server]
pub async fn fetch_revenue_timeseries(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    filter: GroupFilter,
) -> Result<RevenueTimeseries, ServerFnError> {
    use chrono::{Datelike, TimeZone, Timelike};
    if end_date <= start_date {
        return Err(not_found("End date must be after start date"));
    }
    let pool = expect_context::<sqlx::SqlitePool>();

    let duration = end_date.signed_duration_since(start_date);
    let use_hourly = duration <= chrono::Duration::hours(48);

    let mut buckets: Vec<RevenueBucket> = Vec::new();
    if use_hourly {
        let mut t = chrono::Utc
            .with_ymd_and_hms(
                start_date.year(),
                start_date.month(),
                start_date.day(),
                start_date.hour(),
                0,
                0,
            )
            .single()
            .unwrap_or(start_date);
        while t < end_date {
            buckets.push(RevenueBucket {
                bucket_start: t,
                label: t.format("%H:%M").to_string(),
                revenue: 0.0,
            });
            t = t + chrono::Duration::hours(1);
        }
    } else {
        let mut d = start_date.date_naive();
        let end_d = end_date.date_naive();
        while d <= end_d {
            let ts = d
                .and_hms_opt(0, 0, 0)
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(start_date);
            buckets.push(RevenueBucket {
                bucket_start: ts,
                label: d.format("%m-%d").to_string(),
                revenue: 0.0,
            });
            d = d.succ_opt().unwrap_or(d);
        }
    }

    let rows = sqlx::query_as::<_, (f64, DateTime<Utc>)>(&format!(
        "SELECT total, closed_at FROM transactions
         WHERE status = 'closed' AND closed_at >= ? AND closed_at < ?{}",
        group_filter_clause(&filter, "transactions"),
    ))
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    let mut total_revenue = 0.0_f64;
    if !buckets.is_empty() {
        for (total, closed_at) in rows {
            total_revenue += total;
            let idx = if use_hourly {
                let h = closed_at.signed_duration_since(buckets[0].bucket_start).num_hours();
                if h < 0 { continue; }
                h as usize
            } else {
                let d = closed_at.date_naive().signed_duration_since(buckets[0].bucket_start.date_naive()).num_days();
                if d < 0 { continue; }
                d as usize
            };
            if let Some(b) = buckets.get_mut(idx) {
                b.revenue += total;
            }
        }
    }

    Ok(RevenueTimeseries {
        buckets,
        bucket_unit: if use_hourly { "hour".into() } else { "day".into() },
        total_revenue,
    })
}

#[server]
pub async fn fetch_basket_size_distribution(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    filter: GroupFilter,
) -> Result<BasketSizeDistribution, ServerFnError> {
    if end_date <= start_date {
        return Err(not_found("End date must be after start date"));
    }
    let pool = expect_context::<sqlx::SqlitePool>();

    let rows = sqlx::query_as::<_, (Uuid, i64)>(&format!(
        "SELECT t.id, COALESCE(SUM(ti.quantity), 0) as items_count
         FROM transactions t
         LEFT JOIN transaction_items ti ON ti.transaction_id = t.id
         WHERE t.status = 'closed' AND t.closed_at >= ? AND t.closed_at < ?{}
         GROUP BY t.id",
        group_filter_clause(&filter, "t"),
    ))
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    // Bucket boundaries (inclusive lower / inclusive upper, last is open-ended).
    let edges: [(i64, i64, &str); 6] = [
        (1, 1, "1"),
        (2, 2, "2"),
        (3, 3, "3"),
        (4, 5, "4-5"),
        (6, 9, "6-9"),
        (10, i64::MAX, "10+"),
    ];
    let mut counts = [0i64; 6];
    let mut total_items: i64 = 0;
    let mut total_baskets: i64 = 0;
    for (_id, qty) in &rows {
        if *qty <= 0 { continue; }
        total_baskets += 1;
        total_items += qty;
        for (i, (lo, hi, _)) in edges.iter().enumerate() {
            if *qty >= *lo && *qty <= *hi {
                counts[i] += 1;
                break;
            }
        }
    }

    let buckets: Vec<BasketSizeBucket> = edges
        .iter()
        .enumerate()
        .map(|(i, (_, _, label))| BasketSizeBucket {
            label: (*label).to_string(),
            count: counts[i],
        })
        .collect();

    let average_items = if total_baskets > 0 {
        total_items as f64 / total_baskets as f64
    } else {
        0.0
    };

    Ok(BasketSizeDistribution {
        buckets,
        total_transactions: total_baskets,
        average_items,
    })
}

#[server]
pub async fn fetch_payment_analysis(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    filter: GroupFilter,
) -> Result<PaymentAnalysis, ServerFnError> {
    if end_date <= start_date {
        return Err(not_found("End date must be after start date"));
    }
    let pool = expect_context::<sqlx::SqlitePool>();

    let rows = sqlx::query_as::<_, (Option<f64>, Option<f64>)>(&format!(
        "SELECT paid_amount, change_amount FROM transactions
         WHERE status = 'closed' AND closed_at >= ? AND closed_at < ?{}",
        group_filter_clause(&filter, "transactions"),
    ))
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    let edges: [(f64, f64, &str); 6] = [
        (0.0, 0.0, "exact"),
        (0.01, 1.0, "<1"),
        (1.0, 5.0, "1-5"),
        (5.0, 10.0, "5-10"),
        (10.0, 20.0, "10-20"),
        (20.0, f64::INFINITY, "20+"),
    ];
    let mut counts = [0i64; 6];
    let mut total_paid = 0.0_f64;
    let mut total_change = 0.0_f64;
    let mut exact_count = 0i64;
    let mut tx_count = 0i64;

    for (paid, change) in rows {
        let paid = paid.unwrap_or(0.0);
        let change = change.unwrap_or(0.0);
        total_paid += paid;
        total_change += change;
        tx_count += 1;
        if change <= 0.001 {
            exact_count += 1;
        }
        for (i, (lo, hi, _)) in edges.iter().enumerate() {
            // First bucket is "exact" (change == 0); others are ranges.
            if i == 0 {
                if change <= 0.001 {
                    counts[0] += 1;
                    break;
                }
            } else if change >= *lo && (change < *hi || hi.is_infinite()) {
                counts[i] += 1;
                break;
            }
        }
    }

    let average_change = if tx_count > 0 {
        total_change / tx_count as f64
    } else {
        0.0
    };

    let change_distribution: Vec<ChangeBucket> = edges
        .iter()
        .enumerate()
        .map(|(i, (_, _, label))| ChangeBucket {
            label: (*label).to_string(),
            count: counts[i],
        })
        .collect();

    Ok(PaymentAnalysis {
        transaction_count: tx_count,
        total_paid,
        total_change,
        average_change,
        exact_payment_count: exact_count,
        change_distribution,
    })
}

#[server]
pub async fn fetch_underperforming_items(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    limit: i64,
    filter: GroupFilter,
) -> Result<Vec<UnderperformingItem>, ServerFnError> {
    if end_date <= start_date {
        return Err(not_found("End date must be after start date"));
    }
    let pool = expect_context::<sqlx::SqlitePool>();

    let items = sqlx::query_as::<_, UnderperformingItem>(&format!(
        "SELECT i.id as item_id, i.name as item_name,
                COALESCE(c.name, '') as category_name,
                i.price as price,
                CAST(0 AS INTEGER) as quantity_sold,
                CAST(0 AS REAL) as revenue,
                i.created_at as created_at
         FROM items i
         LEFT JOIN categories c ON i.category_id = c.id
         WHERE i.created_at < ?
           AND i.id NOT IN (
               SELECT DISTINCT ti.item_id FROM transaction_items ti
               JOIN transactions t ON ti.transaction_id = t.id
               WHERE t.status = 'closed' AND t.closed_at >= ? AND t.closed_at < ?{}
           )
         ORDER BY i.created_at ASC, i.name ASC
         LIMIT ?",
        group_filter_clause(&filter, "t"),
    ))
    .bind(end_date)
    .bind(start_date)
    .bind(end_date)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    Ok(items)
}

#[server]
pub async fn export_report_csv(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    filter: GroupFilter,
) -> Result<String, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let report = generate_sales_report_db(&pool, start_date, end_date, &filter).await?;

    let mut csv = String::from("Item,Category,Quantity Sold,Revenue,Avg Price,Transactions\n");
    for item in &report.items {
        csv.push_str(&format!(
            "\"{}\",\"{}\",{},{:.2},{:.2},{}\n",
            item.item_name.replace('"', "\"\""),
            item.category_name.replace('"', "\"\""),
            item.quantity_sold,
            item.total_revenue,
            item.average_price,
            item.transaction_count,
        ));
    }
    csv.push_str(&format!(
        "\nTotal,,{},{:.2},,{}\n",
        report.summary.total_items_sold,
        report.summary.total_revenue,
        report.summary.total_transactions,
    ));
    csv.push_str(&format!("Average Transaction Value,,,,{:.2},\n", report.summary.average_transaction_value));
    Ok(csv)
}

// ---- Kitchen Server Functions ----

#[server]
pub async fn fetch_kitchen_orders() -> Result<Vec<KitchenOrder>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct KoRow {
        id: Uuid,
        transaction_id: Uuid,
        transaction_item_id: Uuid,
        item_name: String,
        quantity: i32,
        customer_name: Option<String>,
        completed: bool,
        created_at: DateTime<Utc>,
    }

    let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0)
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        .unwrap_or_else(Utc::now);

    // Fetch ALL items for any order from today that has at least one pending item
    let rows = sqlx::query_as::<_, KoRow>(
        "SELECT * FROM kitchen_order_items
         WHERE created_at >= ? AND transaction_id IN (
             SELECT DISTINCT transaction_id FROM kitchen_order_items WHERE completed = 0 AND created_at >= ?
         )
         ORDER BY created_at ASC",
    )
    .bind(today_start)
    .bind(today_start)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    // Group by transaction_id
    let mut orders: Vec<KitchenOrder> = Vec::new();
    for row in rows {
        let existing = orders.iter_mut().find(|o| o.transaction_id == row.transaction_id);
        let item = KitchenOrderItem {
            transaction_item_id: row.transaction_item_id,
            item_name: row.item_name,
            quantity: row.quantity,
            completed: row.completed,
        };
        if let Some(order) = existing {
            order.items.push(item);
        } else {
            orders.push(KitchenOrder {
                transaction_id: row.transaction_id,
                customer_name: row.customer_name,
                created_at: row.created_at,
                items: vec![item],
            });
        }
    }
    Ok(orders)
}

#[server]
pub async fn complete_kitchen_item(
    transaction_item_id: Uuid,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    sqlx::query(
        "UPDATE kitchen_order_items SET completed = 1, completed_at = ? WHERE transaction_item_id = ?",
    )
    .bind(Utc::now())
    .bind(transaction_item_id)
    .execute(&pool)
    .await
    .map_err(db_err)?;
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<()>>() {
        let _ = tx.send(());
    }
    Ok(())
}

#[server]
pub async fn complete_kitchen_order(
    transaction_id: Uuid,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    sqlx::query(
        "UPDATE kitchen_order_items SET completed = 1, completed_at = ? WHERE transaction_id = ?",
    )
    .bind(Utc::now())
    .bind(transaction_id)
    .execute(&pool)
    .await
    .map_err(db_err)?;
    if let Some(tx) = use_context::<tokio::sync::broadcast::Sender<()>>() {
        let _ = tx.send(());
    }
    Ok(())
}

#[server]
pub async fn fetch_completed_kitchen_orders() -> Result<Vec<KitchenOrder>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct KoRow {
        id: Uuid,
        transaction_id: Uuid,
        transaction_item_id: Uuid,
        item_name: String,
        quantity: i32,
        customer_name: Option<String>,
        completed: bool,
        created_at: DateTime<Utc>,
    }

    let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0)
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        .unwrap_or_else(Utc::now);

    let rows = sqlx::query_as::<_, KoRow>(
        "SELECT * FROM kitchen_order_items WHERE completed = 1 AND created_at >= ? ORDER BY created_at DESC",
    )
    .bind(today_start)
    .fetch_all(&pool)
    .await
    .map_err(db_err)?;

    let mut orders: Vec<KitchenOrder> = Vec::new();
    for row in rows {
        let existing = orders.iter_mut().find(|o| o.transaction_id == row.transaction_id);
        let item = KitchenOrderItem {
            transaction_item_id: row.transaction_item_id,
            item_name: row.item_name,
            quantity: row.quantity,
            completed: row.completed,
        };
        if let Some(order) = existing {
            order.items.push(item);
        } else {
            orders.push(KitchenOrder {
                transaction_id: row.transaction_id,
                customer_name: row.customer_name,
                created_at: row.created_at,
                items: vec![item],
            });
        }
    }
    Ok(orders)
}

// ---- Auth Server Functions ----

#[server]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let user = get_authenticated_user(&pool).await?;
    Ok(user.map(|u| UserInfo {
        id: u.id,
        username: u.username,
        role: u.role,
    }))
}

#[server]
pub async fn fetch_user_list() -> Result<Vec<UserInfo>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let users = sqlx::query_as::<_, UserAccount>("SELECT * FROM users ORDER BY username")
        .fetch_all(&pool)
        .await
        .map_err(db_err)?;
    Ok(users
        .into_iter()
        .map(|u| UserInfo {
            id: u.id,
            username: u.username,
            role: u.role,
        })
        .collect())
}

#[server]
pub async fn login(user_id: Uuid, pin: String) -> Result<UserInfo, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    let user = sqlx::query_as::<_, UserAccount>("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("User not found"))?;

    let pin_hash = hash_pin(&pin);
    if user.pin_hash != pin_hash {
        return Err(not_found("Invalid PIN"));
    }

    let session_id = Uuid::new_v4();
    let token = Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires = now + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, token, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(user.id)
    .bind(&token)
    .bind(now)
    .bind(expires)
    .execute(&pool)
    .await
    .map_err(db_err)?;

    let response_options = expect_context::<leptos_axum::ResponseOptions>();
    response_options.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&format!(
            "rustpos_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000",
            token
        ))
        .unwrap(),
    );

    Ok(UserInfo {
        id: user.id,
        username: user.username,
        role: user.role,
    })
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();

    let Ok(headers) = leptos_axum::extract::<axum::http::HeaderMap>().await else {
        return Ok(());
    };

    if let Some(token) = extract_session_token(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(&token)
            .execute(&pool)
            .await
            .map_err(db_err)?;
    }

    let response_options = expect_context::<leptos_axum::ResponseOptions>();
    response_options.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(
            "rustpos_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        )
        .unwrap(),
    );

    Ok(())
}

#[server]
pub async fn check_initial_setup() -> Result<Option<InitialCredentials>, ServerFnError> {
    let content = std::fs::read_to_string("data/initial_credentials.txt").ok();
    match content {
        Some(s) => {
            let mut lines = s.lines();
            let username = lines.next().unwrap_or("admin").to_string();
            let pin = lines.next().unwrap_or("").to_string();
            Ok(Some(InitialCredentials { username, pin }))
        }
        None => Ok(None),
    }
}

#[server]
pub async fn acknowledge_setup() -> Result<(), ServerFnError> {
    let _ = std::fs::remove_file("data/initial_credentials.txt");
    Ok(())
}

#[server]
pub async fn create_user_account(
    username: String,
    pin: String,
    role: String,
) -> Result<UserInfo, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;

    if !["admin", "cashier", "cook"].contains(&role.as_str()) {
        return Err(not_found("Invalid role"));
    }
    if pin.len() < 4 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(not_found("PIN must be at least 4 digits"));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let pin_hash = hash_pin(&pin);

    let user = sqlx::query_as::<_, UserAccount>(
        "INSERT INTO users (id, username, pin_hash, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(id)
    .bind(&username)
    .bind(&pin_hash)
    .bind(&role)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;

    Ok(UserInfo {
        id: user.id,
        username: user.username,
        role: user.role,
    })
}

#[server]
pub async fn update_user_account(
    id: Uuid,
    username: Option<String>,
    pin: Option<String>,
    role: Option<String>,
) -> Result<UserInfo, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;

    let mut user = sqlx::query_as::<_, UserAccount>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("User not found"))?;

    if let Some(n) = username {
        user.username = n;
    }
    if let Some(p) = pin {
        if p.len() < 4 || !p.chars().all(|c| c.is_ascii_digit()) {
            return Err(not_found("PIN must be at least 4 digits"));
        }
        user.pin_hash = hash_pin(&p);
    }
    if let Some(r) = role {
        if !["admin", "cashier", "cook"].contains(&r.as_str()) {
            return Err(not_found("Invalid role"));
        }
        user.role = r;
    }
    user.updated_at = Utc::now();

    let updated = sqlx::query_as::<_, UserAccount>(
        "UPDATE users SET username = ?, pin_hash = ?, role = ?, updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(&user.username)
    .bind(&user.pin_hash)
    .bind(&user.role)
    .bind(user.updated_at)
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(db_err)?;

    Ok(UserInfo {
        id: updated.id,
        username: updated.username,
        role: updated.role,
    })
}

#[server]
pub async fn delete_user_account(id: Uuid) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let admin = require_admin(&pool).await?;

    if admin.id == id {
        return Err(not_found("Cannot delete your own account"));
    }

    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(db_err)?;

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(not_found("User not found"));
    }
    Ok(())
}

// ---- Config / i18n Server Functions ----

#[server]
pub async fn get_config_language() -> Result<Option<String>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let lang: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'language'")
            .fetch_optional(&pool)
            .await
            .map_err(db_err)?;
    Ok(lang)
}

#[server]
pub async fn set_config_language(lang: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ('language', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&lang)
    .execute(&pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[server]
pub async fn check_system_initialized() -> Result<(bool, bool), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let has_language: bool =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM config WHERE key = 'language'")
            .fetch_one(&pool)
            .await
            .map_err(db_err)?
            > 0;
    let has_users: bool =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .map_err(db_err)?
            > 0;
    Ok((has_language, has_users))
}

#[server]
pub async fn initialize_admin() -> Result<InitialCredentials, ServerFnError> {
    // Generate random PIN before any awaits (thread_rng is !Send)
    let pin_str = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let pin_num: u32 = rng.gen_range(1000..10000);
        format!("{:04}", pin_num)
    };

    let pool = expect_context::<sqlx::SqlitePool>();

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .map_err(db_err)?;

    if user_count > 0 {
        return Err(not_found("Users already exist"));
    }
    let pin_hash_val = hash_pin(&pin_str);

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO users (id, username, pin_hash, role, created_at, updated_at) VALUES (?, ?, ?, 'admin', ?, ?)",
    )
    .bind(id)
    .bind("admin")
    .bind(&pin_hash_val)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .map_err(db_err)?;

    // Print credentials if printer available
    {
        use crate::printer::{find_printer, print_credentials};
        let pin_for_print = pin_str.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok((_path, mut printer)) = find_printer() {
                let _ = print_credentials(&mut printer, "admin", &pin_for_print);
            }
        })
        .await;
    }

    Ok(InitialCredentials {
        username: "admin".to_string(),
        pin: pin_str,
    })
}

#[server]
pub async fn set_language_admin(lang: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ('language', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&lang)
    .execute(&pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[server]
pub async fn get_config_currency() -> Result<Option<String>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    let currency: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'currency'")
            .fetch_optional(&pool)
            .await
            .map_err(db_err)?;
    Ok(currency)
}

#[server]
pub async fn set_config_currency(currency: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ('currency', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&currency)
    .execute(&pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[server]
pub async fn set_currency_admin(currency: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ('currency', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&currency)
    .execute(&pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[server]
pub async fn get_printer_passphrase_set() -> Result<bool, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    let exists: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'printer_passphrase'")
            .fetch_optional(&pool)
            .await
            .map_err(db_err)?;
    Ok(exists.is_some())
}

#[server]
pub async fn set_printer_passphrase(passphrase: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    if passphrase.len() < 8 {
        return Err(not_found("Passphrase must be at least 8 characters"));
    }
    let hash = hash_pin(&passphrase);
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ('printer_passphrase', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&hash)
    .execute(&pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[server]
pub async fn clear_printer_passphrase() -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    sqlx::query("DELETE FROM config WHERE key = 'printer_passphrase'")
        .execute(&pool)
        .await
        .map_err(db_err)?;
    Ok(())
}

#[server]
pub async fn get_printer_codepage() -> Result<u8, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'printer_codepage'")
            .fetch_optional(&pool)
            .await
            .map_err(db_err)?;
    let page = value
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(crate::printer::DEFAULT_CODEPAGE);
    Ok(page)
}

#[server]
pub async fn set_printer_codepage(codepage: u8) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    if codepage == 0 {
        return Err(not_found("Code page must be between 1 and 255"));
    }
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ('printer_codepage', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(codepage.to_string())
    .execute(&pool)
    .await
    .map_err(db_err)?;
    // Apply immediately so it takes effect without restarting the server.
    crate::printer::set_codepage(codepage);
    Ok(())
}

/// Reads the "disable local receipt printing" flag (default: false / enabled).
/// No auth check — used internally at checkout time.
#[cfg(feature = "ssr")]
async fn read_disable_local_printing(pool: &sqlx::SqlitePool) -> bool {
    sqlx::query_scalar::<_, String>("SELECT value FROM config WHERE key = 'disable_local_printing'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false)
}

#[server]
pub async fn get_disable_local_printing() -> Result<bool, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    Ok(read_disable_local_printing(&pool).await)
}

#[server]
pub async fn set_disable_local_printing(disabled: bool) -> Result<(), ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ('disable_local_printing', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(if disabled { "true" } else { "false" })
    .execute(&pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Print a sales breakdown for the given period on the local printer:
/// per-item quantity sold and total sale value, plus period totals.
#[server]
pub async fn print_sales_report(
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    filter: GroupFilter,
) -> Result<(), ServerFnError> {
    use crate::printer::{find_printer, print_sales_report as print_sr};

    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;

    let report = generate_sales_report_db(&pool, start_date, end_date, &filter).await?;
    let currency: String =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'currency'")
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();

    let items: Vec<(String, u32, f32)> = report
        .items
        .iter()
        .map(|i| (i.item_name.clone(), i.quantity_sold as u32, i.total_revenue as f32))
        .collect();
    let total_items_sold = report.summary.total_items_sold as u32;
    let total_revenue = report.summary.total_revenue as f32;
    let period = format!(
        "{} to {}",
        start_date.format("%Y-%m-%d"),
        end_date.format("%Y-%m-%d")
    );
    let now = chrono::Local::now();

    let result: Result<(), String> = tokio::task::spawn_blocking(move || {
        let (_, mut printer) = find_printer().map_err(|e| e.to_string())?;
        print_sr(
            &mut printer,
            &period,
            &currency,
            items,
            total_items_sold,
            total_revenue,
            now,
            Some("data/logo_receipt.png"),
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    result.map_err(ServerFnError::new)?;
    Ok(())
}

/// Returns the system's network addresses as "ip (interface)" strings,
/// loopback excluded, IPv4 listed before IPv6. Useful for pointing remote
/// print clients and other devices at this server.
#[server]
pub async fn get_system_ip_addresses() -> Result<Vec<String>, ServerFnError> {
    let pool = expect_context::<sqlx::SqlitePool>();
    require_admin(&pool).await?;

    // Same port resolution as the server bind in main.rs.
    let port = std::env::var("RUSTPOS_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let mut addrs: Vec<(bool, String)> = if_addrs::get_if_addrs()
        .map_err(db_err)?
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .map(|iface| {
            let ip = iface.ip();
            // IPv6 addresses must be bracketed in a URL.
            let host = if ip.is_ipv6() { format!("[{}]", ip) } else { ip.to_string() };
            (ip.is_ipv6(), format!("http://{}:{} ({})", host, port, iface.name))
        })
        .collect();
    // IPv4 first (is_ipv6 == false), then by string for stable ordering.
    addrs.sort();
    Ok(addrs.into_iter().map(|(_, s)| s).collect())
}
