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
