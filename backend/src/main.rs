use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, FromRow};
use std::{net::SocketAddr, env};
use tower_http::cors::CorsLayer;
use uuid::Uuid;
use tower_http::services::{ServeDir, ServeFile};

mod printer;
use printer::{find_printer, print_receipt};

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Not found")]
    NotFound,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

// Domain Models
#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Category {
    id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// Report Models
#[derive(Debug, Serialize, FromRow)]
struct ItemSalesReport {
    item_id: Uuid,
    item_name: String,
    category_name: String,
    quantity_sold: i64,
    total_revenue: f64,
    average_price: f64,
    transaction_count: i64,
}

#[derive(Debug, Serialize)]
struct SalesReport {
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    items: Vec<ItemSalesReport>,
    summary: ReportSummary,
}

#[derive(Debug, Serialize)]
struct ReportSummary {
    total_revenue: f64,
    total_items_sold: i64,
    total_transactions: i64,
    average_transaction_value: f64,
    top_selling_item: Option<String>,
    top_revenue_item: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReportDateRange {
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Item {
    id: Uuid,
    name: String,
    description: Option<String>,
    price: f64,
    category_id: Uuid,
    sku: Option<String>,
    in_stock: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Transaction {
    id: Uuid,
    customer_name: Option<String>,
    status: String, // "open", "closed", "cancelled"
    total: f64,
    paid_amount: Option<f64>,
    change_amount: Option<f64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct TransactionItem {
    id: Uuid,
    transaction_id: Uuid,
    item_id: Uuid,
    quantity: i32,
    unit_price: f64,
    total_price: f64,
    created_at: DateTime<Utc>,
}

// DTOs
#[derive(Debug, Deserialize)]
struct CreateCategoryDto {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateCategoryDto {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateItemDto {
    name: String,
    description: Option<String>,
    price: f64,
    category_id: Uuid,
    sku: Option<String>,
    in_stock: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateItemDto {
    name: Option<String>,
    description: Option<String>,
    price: Option<f64>,
    category_id: Option<Uuid>,
    sku: Option<String>,
    in_stock: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CreateTransactionDto {
    customer_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateTransactionDto {
    customer_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddTransactionItemDto {
    item_id: Uuid,
    quantity: i32,
}

#[derive(Debug, Deserialize)]
struct UpdateTransactionItemDto {
    item_id: Uuid,
    quantity: i32,
}

#[derive(Debug, Deserialize)]
struct CloseTransactionDto {
    paid_amount: f64,
}

#[derive(Debug, Serialize)]
struct CloseTransactionResponse {
    transaction: Transaction,
    change_amount: f64,
}

#[derive(Debug, Serialize)]
struct TransactionDetailsResponse {
    transaction: Transaction,
    items: Vec<TransactionItemDetail>,
}

#[derive(Debug, Serialize, FromRow)]
struct TransactionItemDetail {
    id: Uuid,
    item_id: Uuid,
    item_name: String,
    quantity: i32,
    unit_price: f64,
    total_price: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Look for POS printer on any serial or USB port
    println!("Searching for POS printer...");
    match find_printer() {
        Ok((path, printer)) => {
            println!("Found printer at: {}", path);
        }
        Err(e) => {
            eprintln!("Error finding printer: {}", e);
        }
    }

    // Create database connection with auto-create
    let db = SqlitePool::connect("sqlite:data/pos.db?mode=rwc").await?;

    // Run migrations inline (no separate files needed)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS categories (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"#
    )
    .execute(&db)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            price REAL NOT NULL,
            category_id TEXT NOT NULL,
            sku TEXT,
            in_stock BOOLEAN NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (category_id) REFERENCES categories(id)
        )"#
    )
    .execute(&db)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS transactions (
            id TEXT PRIMARY KEY,
            customer_name TEXT,
            status TEXT NOT NULL CHECK (status IN ('open', 'closed', 'cancelled')),
            total REAL NOT NULL DEFAULT 0,
            paid_amount REAL,
            change_amount REAL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT
        )"#
    )
    .execute(&db)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS transaction_items (
            id TEXT PRIMARY KEY,
            transaction_id TEXT NOT NULL,
            item_id TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            unit_price REAL NOT NULL,
            total_price REAL NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE,
            FOREIGN KEY (item_id) REFERENCES items(id)
        )"#
    )
    .execute(&db)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_category_id ON items(category_id)")
        .execute(&db)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_transaction_items_transaction_id ON transaction_items(transaction_id)")
        .execute(&db)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_transaction_items_item_id ON transaction_items(item_id)")
        .execute(&db)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status)")
        .execute(&db)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_customer_name ON transactions(customer_name)")
        .execute(&db)
        .await?;

    println!("Database initialized successfully!");

    let state = AppState { db };

    // Build router
    let api_routes = Router::new()
        // Category routes
        .route("/categories", get(get_categories).post(create_category))
        .route(
            "/categories/{id}",
            get(get_category)
                .put(update_category)
                .delete(delete_category),
        )
        // Item routes
        .route("/items", get(get_items).post(create_item))
        .route(
            "/items/{id}",
            get(get_item).put(update_item).delete(delete_item),
        )
        .route("/items/category/{category_id}", get(get_items_by_category))
        // Transaction routes
        .route("/transactions", get(get_transactions).post(create_transaction))
        .route("/transactions/{id}", get(get_transaction).put(update_transaction))
        .route("/transactions/{id}/items", post(add_transaction_item))
        .route("/transactions/{id}/items/{item_id}", delete(remove_transaction_item).put(update_transaction_item))
        .route("/transactions/{id}/close", post(close_transaction))
        .route("/transactions/{id}/cancel", post(cancel_transaction))
        .route("/transactions/open", get(get_open_transactions))
        // Report routes
        .route("/reports/sales", post(generate_sales_report))
        .route("/reports/daily", get(get_daily_report))
        .route("/reports/monthly", get(get_monthly_report))
        .with_state(state);

    // Serve frontend files
    let serve_dir = ServeDir::new("static")
        .not_found_service(ServeFile::new("static/index.html"));

    // Combine API and frontend
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(serve_dir)
        .layer(CorsLayer::permissive());

    let port = env::var("RUSTPOS_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>()?;
    println!("RustPOS is accessible on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Category handlers
async fn get_categories(State(state): State<AppState>) -> Result<Json<Vec<Category>>> {
    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY name")
        .fetch_all(&state.db)
        .await?;
    
    Ok(Json(categories))
}

async fn get_category(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Category>> {
    let category = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    
    Ok(Json(category))
}

async fn create_category(
    State(state): State<AppState>,
    Json(dto): Json<CreateCategoryDto>,
) -> Result<(StatusCode, Json<Category>)> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    let category = sqlx::query_as::<_, Category>(
        "INSERT INTO categories (id, name, description, created_at, updated_at) 
         VALUES (?, ?, ?, ?, ?) 
         RETURNING *"
    )
    .bind(id)
    .bind(&dto.name)
    .bind(&dto.description)
    .bind(now)
    .bind(now)
    .fetch_one(&state.db)
    .await?;
    
    Ok((StatusCode::CREATED, Json(category)))
}

async fn update_category(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateCategoryDto>,
) -> Result<Json<Category>> {
    let mut category = sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    
    if let Some(name) = dto.name {
        category.name = name;
    }
    if let Some(desc) = dto.description {
        category.description = Some(desc);
    }
    category.updated_at = Utc::now();
    
    let updated = sqlx::query_as::<_, Category>(
        "UPDATE categories SET name = ?, description = ?, updated_at = ? 
         WHERE id = ? RETURNING *"
    )
    .bind(&category.name)
    .bind(&category.description)
    .bind(category.updated_at)
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    
    Ok(Json(updated))
}

async fn delete_category(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let result = sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    
    Ok(StatusCode::NO_CONTENT)
}

// Item handlers
async fn get_items(State(state): State<AppState>) -> Result<Json<Vec<Item>>> {
    let items = sqlx::query_as::<_, Item>("SELECT * FROM items ORDER BY name")
        .fetch_all(&state.db)
        .await?;
    
    Ok(Json(items))
}

async fn get_items_by_category(
    State(state): State<AppState>,
    Path(category_id): Path<Uuid>,
) -> Result<Json<Vec<Item>>> {
    let items = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE category_id = ? ORDER BY name")
        .bind(category_id)
        .fetch_all(&state.db)
        .await?;
    
    Ok(Json(items))
}

async fn get_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Item>> {
    let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    
    Ok(Json(item))
}

async fn create_item(
    State(state): State<AppState>,
    Json(dto): Json<CreateItemDto>,
) -> Result<(StatusCode, Json<Item>)> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let in_stock = dto.in_stock.unwrap_or(true);
    
    let item = sqlx::query_as::<_, Item>(
        "INSERT INTO items (id, name, description, price, category_id, sku, in_stock, created_at, updated_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) 
         RETURNING *"
    )
    .bind(id)
    .bind(&dto.name)
    .bind(&dto.description)
    .bind(dto.price)
    .bind(dto.category_id)
    .bind(&dto.sku)
    .bind(in_stock)
    .bind(now)
    .bind(now)
    .fetch_one(&state.db)
    .await?;
    
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateItemDto>,
) -> Result<Json<Item>> {
    let mut item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    
    if let Some(name) = dto.name {
        item.name = name;
    }
    if let Some(desc) = dto.description {
        item.description = Some(desc);
    }
    if let Some(price) = dto.price {
        item.price = price;
    }
    if let Some(cat_id) = dto.category_id {
        item.category_id = cat_id;
    }
    if let Some(sku) = dto.sku {
        item.sku = Some(sku);
    }
    if let Some(in_stock) = dto.in_stock {
        item.in_stock = in_stock;
    }
    item.updated_at = Utc::now();
    
    let updated = sqlx::query_as::<_, Item>(
        "UPDATE items SET name = ?, description = ?, price = ?, category_id = ?, 
         sku = ?, in_stock = ?, updated_at = ? 
         WHERE id = ? RETURNING *"
    )
    .bind(&item.name)
    .bind(&item.description)
    .bind(item.price)
    .bind(item.category_id)
    .bind(&item.sku)
    .bind(item.in_stock)
    .bind(item.updated_at)
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    
    Ok(Json(updated))
}

async fn delete_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let result = sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    
    Ok(StatusCode::NO_CONTENT)
}

// Transaction handlers
async fn get_transactions(State(state): State<AppState>) -> Result<Json<Vec<Transaction>>> {
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(transactions))
}

async fn get_open_transactions(State(state): State<AppState>) -> Result<Json<Vec<Transaction>>> {
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE status = 'open' ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(transactions))
}

async fn get_transaction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TransactionDetailsResponse>> {
    let transaction = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    
    let items = sqlx::query_as::<_, TransactionItemDetail>(
        "SELECT ti.id, ti.item_id, i.name as item_name, ti.quantity, 
         ti.unit_price, ti.total_price 
         FROM transaction_items ti 
         JOIN items i ON ti.item_id = i.id 
         WHERE ti.transaction_id = ?"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    
    Ok(Json(TransactionDetailsResponse { transaction, items }))
}

async fn create_transaction(
    State(state): State<AppState>,
    Json(dto): Json<CreateTransactionDto>,
) -> Result<(StatusCode, Json<Transaction>)> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    let transaction = sqlx::query_as::<_, Transaction>(
        "INSERT INTO transactions (id, customer_name, status, total, created_at, updated_at) 
         VALUES (?, ?, 'open', 0.0, ?, ?) 
         RETURNING *"
    )
    .bind(id)
    .bind(&dto.customer_name)
    .bind(now)
    .bind(now)
    .fetch_one(&state.db)
    .await?;
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

async fn add_transaction_item(
    State(state): State<AppState>,
    Path(transaction_id): Path<Uuid>,
    Json(dto): Json<AddTransactionItemDto>,
) -> Result<(StatusCode, Json<TransactionItem>)> {
    // Check transaction exists and is open
    let _transaction = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'"
    )
    .bind(transaction_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("Transaction not found or not open".to_string()))?;
    
    // Get item details
    let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
        .bind(dto.item_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    
    if !item.in_stock {
        return Err(AppError::BadRequest("Item is out of stock".to_string()));
    }
    
    let id = Uuid::new_v4();
    let total_price = item.price * dto.quantity as f64;
    let now = Utc::now();
    
    // Insert transaction item
    let transaction_item = sqlx::query_as::<_, TransactionItem>(
        "INSERT INTO transaction_items (id, transaction_id, item_id, quantity, unit_price, total_price, created_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?) 
         RETURNING *"
    )
    .bind(id)
    .bind(transaction_id)
    .bind(dto.item_id)
    .bind(dto.quantity)
    .bind(item.price)
    .bind(total_price)
    .bind(now)
    .fetch_one(&state.db)
    .await?;
    
    // Update transaction total
    update_transaction_total(&state.db, transaction_id).await?;
    
    Ok((StatusCode::CREATED, Json(transaction_item)))
}

async fn update_transaction_item(
    State(state): State<AppState>,
    Path((transaction_id, item_id)): Path<(Uuid, Uuid)>,
    Json(dto): Json<UpdateTransactionItemDto>,
) -> Result<Json<TransactionItem>> {
    // Only allow update if transaction is open
    sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'"
    )
    .bind(transaction_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("Transaction not found or not open".to_string()))?;

    // Get item details
    let item = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    if !item.in_stock {
        return Err(AppError::BadRequest("Item is out of stock".to_string()));
    }

    // Update transaction item quantity and total price
    let total_price = item.price * dto.quantity as f64;
    let updated = sqlx::query_as::<_, TransactionItem>(
        "UPDATE transaction_items SET quantity = ?, unit_price = ?, total_price = ?, created_at = ? 
         WHERE transaction_id = ? AND item_id = ? RETURNING *"
    )
    .bind(dto.quantity)
    .bind(item.price)
    .bind(total_price)
    .bind(Utc::now())
    .bind(transaction_id)
    .bind(item_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Update transaction total
    update_transaction_total(&state.db, transaction_id).await?;

    Ok(Json(updated))
}

async fn remove_transaction_item(
    State(state): State<AppState>,
    Path((transaction_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    // Check transaction is open
    sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'"
    )
    .bind(transaction_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("Transaction not found or not open".to_string()))?;
    
    let result = sqlx::query(
        "DELETE FROM transaction_items WHERE transaction_id = ? AND item_id = ?"
    )
    .bind(transaction_id)
    .bind(item_id)
    .execute(&state.db)
    .await?;
    
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    
    // Update transaction total
    update_transaction_total(&state.db, transaction_id).await?;
    
    Ok(StatusCode::NO_CONTENT)
}

async fn update_transaction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateTransactionDto>,
) -> Result<Json<Transaction>> {
    // Only allow update if transaction is open
    let _transaction = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("Transaction not found or not open".to_string()))?;

    let updated = sqlx::query_as::<_, Transaction>(
        "UPDATE transactions SET customer_name = ?, updated_at = ? WHERE id = ? RETURNING *"
    )
    .bind(&dto.customer_name)
    .bind(Utc::now())
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated))
}

async fn close_transaction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<CloseTransactionDto>,
) -> Result<Json<CloseTransactionResponse>> {
    let mut transaction = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = ? AND status = 'open'"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("Transaction not found or not open".to_string()))?;
    
    if dto.paid_amount < transaction.total {
        return Err(AppError::BadRequest("Insufficient payment amount".to_string()));
    }
    
    let change = dto.paid_amount - transaction.total;
    let now = Utc::now();
    
    transaction = sqlx::query_as::<_, Transaction>(
        "UPDATE transactions 
         SET status = 'closed', paid_amount = ?, change_amount = ?, 
             closed_at = ?, updated_at = ? 
         WHERE id = ? 
         RETURNING *"
    )
    .bind(dto.paid_amount)
    .bind(change)
    .bind(now)
    .bind(now)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    if transaction.status == "closed" {
    let items = sqlx::query_as::<_, TransactionItemDetail>(
                     "SELECT ti.id, ti.item_id, i.name as item_name, ti.quantity, 
                      ti.unit_price, ti.total_price 
                      FROM transaction_items ti 
                      JOIN items i ON ti.item_id = i.id 
                      WHERE ti.transaction_id = ?"
                 )
        .bind(id)
        .fetch_all(&state.db)
        .await?;

    let receipt_items: Vec<(String, u32, f32)> = items.into_iter()
        .map(|it| (it.item_name, it.quantity as u32, it.unit_price as f32))
        .collect();

    // spawn_blocking runs on a dedicated thread pool
    let _ = tokio::task::spawn_blocking(move || {
        if let Ok((_, mut printer)) = find_printer() {
            let _ = print_receipt(&mut printer, receipt_items, dto.paid_amount as f32, change as f32);
        }
    })
    .await; // JoinHandle is Send; we didn't move the printer across .await
    }

    Ok(Json(CloseTransactionResponse {
        transaction,
        change_amount: change,
    }))
}

async fn cancel_transaction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Transaction>> {
    let transaction = sqlx::query_as::<_, Transaction>(
        "UPDATE transactions 
         SET status = 'cancelled', updated_at = ? 
         WHERE id = ? AND status = 'open' 
         RETURNING *"
    )
    .bind(Utc::now())
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("Transaction not found or not open".to_string()))?;
    
    Ok(Json(transaction))
}

// Helper functions
async fn update_transaction_total(db: &SqlitePool, transaction_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE transactions 
         SET total = (
             SELECT COALESCE(SUM(total_price), 0) 
             FROM transaction_items 
             WHERE transaction_id = ?
         ),
         updated_at = ?
         WHERE id = ?"
    )
    .bind(transaction_id)
    .bind(Utc::now())
    .bind(transaction_id)
    .execute(db)
    .await?;
    
    Ok(())
}

// Report handlers
async fn generate_sales_report(
    State(state): State<AppState>,
    Json(date_range): Json<ReportDateRange>,
) -> Result<Json<SalesReport>> {
    // Validate date range
    if date_range.end_date <= date_range.start_date {
        return Err(AppError::BadRequest("End date must be after start date".to_string()));
    }
    
    // Get item sales data
    let items = sqlx::query_as::<_, ItemSalesReport>(
        "SELECT 
            i.id as item_id,
            i.name as item_name,
            c.name as category_name,
            SUM(ti.quantity) as quantity_sold,
            SUM(ti.total_price) as total_revenue,
            AVG(ti.unit_price) as average_price,
            COUNT(DISTINCT ti.transaction_id) as transaction_count
        FROM transaction_items ti
        JOIN items i ON ti.item_id = i.id
        JOIN categories c ON i.category_id = c.id
        JOIN transactions t ON ti.transaction_id = t.id
        WHERE t.status = 'closed' 
            AND t.closed_at >= ?
            AND t.closed_at < ?
        GROUP BY i.id, i.name, c.name
        ORDER BY total_revenue DESC"
    )
    .bind(date_range.start_date)
    .bind(date_range.end_date)
    .fetch_all(&state.db)
    .await?;
    
    // Calculate summary statistics
    let total_revenue: f64 = items.iter().map(|i| i.total_revenue).sum();
    let total_items_sold: i64 = items.iter().map(|i| i.quantity_sold).sum();
    
    // Get total number of transactions
    let transaction_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT id) FROM transactions 
         WHERE status = 'closed' 
            AND closed_at >= ? 
            AND closed_at < ?"
    )
    .bind(date_range.start_date)
    .bind(date_range.end_date)
    .fetch_one(&state.db)
    .await?;
    
    let average_transaction_value = if transaction_count > 0 {
        total_revenue / transaction_count as f64
    } else {
        0.0
    };
    
    // Find top selling items
    let top_selling_item = items.iter()
        .max_by_key(|i| i.quantity_sold)
        .map(|i| i.item_name.clone());
    
    let top_revenue_item = items.iter()
        .max_by(|a, b| a.total_revenue.partial_cmp(&b.total_revenue).unwrap())
        .map(|i| i.item_name.clone());
    
    let summary = ReportSummary {
        total_revenue,
        total_items_sold,
        total_transactions: transaction_count,
        average_transaction_value,
        top_selling_item,
        top_revenue_item,
    };
    
    Ok(Json(SalesReport {
        start_date: date_range.start_date,
        end_date: date_range.end_date,
        items,
        summary,
    }))
}

async fn get_daily_report(State(state): State<AppState>) -> Result<Json<SalesReport>> {
    let end_date = Utc::now();
    let start_date = end_date - chrono::Duration::days(1);
    
    generate_sales_report(
        State(state),
        Json(ReportDateRange { start_date, end_date })
    ).await
}

async fn get_monthly_report(State(state): State<AppState>) -> Result<Json<SalesReport>> {
    let end_date = Utc::now();
    let start_date = end_date - chrono::Duration::days(30);
    
    generate_sales_report(
        State(state),
        Json(ReportDateRange { start_date, end_date })
    ).await
}
