use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub image_path: Option<String>,
    pub stock_quantity: Option<i32>,
    pub kitchen_item: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesBucket {
    pub bucket_start: DateTime<Utc>,
    pub label: String,
    pub quantities: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSalesTimeseries {
    pub item_ids: Vec<Uuid>,
    pub item_names: Vec<String>,
    pub buckets: Vec<TimeseriesBucket>,
    pub bucket_unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueBucket {
    pub bucket_start: DateTime<Utc>,
    pub label: String,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueTimeseries {
    pub buckets: Vec<RevenueBucket>,
    pub bucket_unit: String,
    pub total_revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketSizeBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketSizeDistribution {
    pub buckets: Vec<BasketSizeBucket>,
    pub total_transactions: i64,
    pub average_items: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAnalysis {
    pub transaction_count: i64,
    pub total_paid: f64,
    pub total_change: f64,
    pub average_change: f64,
    pub exact_payment_count: i64,
    pub change_distribution: Vec<ChangeBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct UnderperformingItem {
    pub item_id: Uuid,
    pub item_name: String,
    pub category_name: String,
    pub price: f64,
    pub quantity_sold: i64,
    pub revenue: f64,
    pub created_at: DateTime<Utc>,
}

// User / Auth models

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct UserAccount {
    pub id: Uuid,
    pub username: String,
    pub pin_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialCredentials {
    pub username: String,
    pub pin: String,
}

// Kitchen models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenOrderItem {
    pub transaction_item_id: Uuid,
    pub item_name: String,
    pub quantity: i32,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenOrder {
    pub transaction_id: Uuid,
    pub customer_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub items: Vec<KitchenOrderItem>,
}
