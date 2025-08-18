use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes, A},
    StaticSegment,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use gloo_net::http::Request;

// API Configuration
const API_BASE: &str = "/api";
const CURRENCY_SYMBOL: &str = "€";

// Shared Models (matching backend)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Category {
    id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    id: Uuid,
    customer_name: Option<String>,
    status: String,
    total: f64,
    paid_amount: Option<f64>,
    change_amount: Option<f64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionItemDetail {
    id: Uuid,
    item_id: Uuid,
    item_name: String,
    quantity: i32,
    unit_price: f64,
    total_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionDetailsResponse {
    transaction: Transaction,
    items: Vec<TransactionItemDetail>,
}

// Report Models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ItemSalesReport {
    item_id: Uuid,
    item_name: String,
    category_name: String,
    quantity_sold: i64,
    total_revenue: f64,
    average_price: f64,
    transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReportSummary {
    total_revenue: f64,
    total_items_sold: i64,
    total_transactions: i64,
    average_transaction_value: f64,
    top_selling_item: Option<String>,
    top_revenue_item: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SalesReport {
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    items: Vec<ItemSalesReport>,
    summary: ReportSummary,
}

#[derive(Debug, Serialize)]
struct ReportDateRange {
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
}

// DTOs
#[derive(Debug, Serialize)]
struct CreateCategoryDto {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateCategoryDto {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateItemDto {
    name: String,
    description: Option<String>,
    price: f64,
    category_id: Uuid,
    sku: Option<String>,
    in_stock: Option<bool>,
}

#[derive(Debug, Serialize)]
struct UpdateItemDto {
    name: Option<String>,
    description: Option<String>,
    price: Option<f64>,
    category_id: Option<Uuid>,
    sku: Option<String>,
    in_stock: Option<bool>,
}

#[derive(Debug, Serialize)]
struct CreateTransactionDto {
    customer_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddTransactionItemDto {
    item_id: Uuid,
    quantity: i32,
}

#[derive(Debug, Serialize)]
struct UpdateTransactionDto {
    customer_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateTransactionItemDto {
    item_id: Uuid,
    quantity: i32,
}

#[derive(Debug, Serialize)]
struct CloseTransactionDto {
    paid_amount: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CloseTransactionResponse {
    transaction: Transaction,
    change_amount: f64,
}

// API Client - Categories
async fn fetch_categories() -> Result<Vec<Category>, String> {
    Request::get(&format!("{}/categories", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn create_category(dto: CreateCategoryDto) -> Result<Category, String> {
    Request::post(&format!("{}/categories", API_BASE))
        .json(&dto)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn update_category(id: Uuid, dto: UpdateCategoryDto) -> Result<Category, String> {
    Request::put(&format!("{}/categories/{}", API_BASE, id))
        .json(&dto)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_category(id: Uuid) -> Result<(), String> {
    Request::delete(&format!("{}/categories/{}", API_BASE, id))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// API Client - Items
async fn fetch_items() -> Result<Vec<Item>, String> {
    Request::get(&format!("{}/items", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn create_item(dto: CreateItemDto) -> Result<Item, String> {
    Request::post(&format!("{}/items", API_BASE))
        .json(&dto)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn update_item(id: Uuid, dto: UpdateItemDto) -> Result<Item, String> {
    Request::put(&format!("{}/items/{}", API_BASE, id))
        .json(&dto)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_item(id: Uuid) -> Result<(), String> {
    Request::delete(&format!("{}/items/{}", API_BASE, id))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// API Client - Transactions
async fn fetch_all_transactions() -> Result<Vec<Transaction>, String> {
    Request::get(&format!("{}/transactions", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_open_transactions() -> Result<Vec<Transaction>, String> {
    Request::get(&format!("{}/transactions/open", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_transaction_details(id: Uuid) -> Result<TransactionDetailsResponse, String> {
    Request::get(&format!("{}/transactions/{}", API_BASE, id))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn create_transaction(customer_name: Option<String>) -> Result<Transaction, String> {
    Request::post(&format!("{}/transactions", API_BASE))
        .json(&CreateTransactionDto { customer_name })
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn update_transaction(id: Uuid, customer_name: Option<String>) -> Result<Transaction, String> {
    Request::put(&format!("{}/transactions/{}", API_BASE, id))
        .json(&UpdateTransactionDto { customer_name })
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn add_item_to_transaction(transaction_id: Uuid, item_id: Uuid, quantity: i32) -> Result<(), String> {
    // Fetch current transaction details
    let details = fetch_transaction_details(transaction_id).await.map_err(|e| e.to_string())?;
    let existing = details.items.iter().find(|item| item.item_id == item_id);

    let new_quantity = if let Some(item) = existing {
        item.quantity + quantity
    } else {
        quantity
    };

    if new_quantity <= 0 {
        // Remove item if quantity is zero or less
        remove_item_from_transaction(transaction_id, item_id).await
    } else if new_quantity == 1 {
        // add item with quantity 1
    Request::post(&format!("{}/transactions/{}/items", API_BASE, transaction_id))
            .json(&AddTransactionItemDto { item_id, quantity: new_quantity })
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
    } else if new_quantity > 1 {
        // Update item quantity
        Request::put(&format!("{}/transactions/{}/items/{}", API_BASE, transaction_id, item_id))
            .json(&UpdateTransactionItemDto { item_id, quantity: new_quantity })
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Invalid quantity".to_string())
    }

}

async fn remove_item_from_transaction(transaction_id: Uuid, item_id: Uuid) -> Result<(), String> {
    // Fetch current transaction details
    let details = fetch_transaction_details(transaction_id).await.map_err(|e| e.to_string())?;
    if let Some(item) = details.items.iter().find(|item| item.item_id == item_id) {
        if item.quantity > 1 {
            // Decrease quantity by 1
            Request::put(&format!("{}/transactions/{}/items/{}", API_BASE, transaction_id, item_id))
                .json(&UpdateTransactionItemDto { item_id, quantity: item.quantity - 1 })
                .map_err(|e| e.to_string())?
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        } else if item.quantity == 1 {
            // Remove item if quantity is 1
    Request::delete(&format!("{}/transactions/{}/items/{}", API_BASE, transaction_id, item_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

async fn close_transaction(id: Uuid, paid_amount: f64) -> Result<CloseTransactionResponse, String> {
    Request::post(&format!("{}/transactions/{}/close", API_BASE, id))
        .json(&CloseTransactionDto { paid_amount })
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn cancel_transaction(id: Uuid) -> Result<Transaction, String> {
    Request::post(&format!("{}/transactions/{}/cancel", API_BASE, id))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

// API Client - Reports
async fn fetch_sales_report(start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<SalesReport, String> {
    Request::post(&format!("{}/reports/sales", API_BASE))
        .json(&ReportDateRange { start_date, end_date })
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_daily_report() -> Result<SalesReport, String> {
    Request::get(&format!("{}/reports/daily", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_monthly_report() -> Result<SalesReport, String> {
    Request::get(&format!("{}/reports/monthly", API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

// Components
#[component]
fn App() -> impl IntoView {
    provide_meta_context();
    
    view! {
        <Html attr:lang="en" />
        <Stylesheet id="leptos" href="/style/main.css"/>
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
    let (transaction_items, set_transaction_items) = signal(Vec::<TransactionItemDetail>::new());
    let (customer_name, set_customer_name) = signal(String::new());
    let (_payment_amount, _set_payment_amount) = signal(String::new());
    let (change_amount, set_change_amount) = signal(Option::<f64>::None);
    let (open_transactions, set_open_transactions) = signal(Vec::<Transaction>::new());
    let (show_open_transactions, set_show_open_transactions) = signal(false);
    let (payment_amount, set_payment_amount) = signal(String::new());
    let (canceling_transaction, set_canceling_transaction) = signal(Option::<Uuid>::None);
    let (last_closed_transaction, set_last_closed_transaction) = signal(Option::<Transaction>::None);

    // Helper to fetch last closed transaction
    let fetch_last_closed_transaction = move || {
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
            if let Ok(items) = fetch_items().await {
                set_items.set(items);
            }
            if let Ok(trans) = fetch_open_transactions().await {
                set_open_transactions.set(trans);
            }
        });
    });
    
    let filtered_items = move || {
        let all_items = items.get();
        match selected_category.get() {
            Some(cat_id) => all_items.into_iter()
                .filter(|item| item.category_id == cat_id)
                .collect(),
            None => all_items,
        }
    };
    
    let transaction_total = move || {
        transaction_items.get().iter().map(|i| i.total_price).sum::<f64>()
    };
    
    let start_transaction = move |_| {
        let name = customer_name.get();
        let set_current_transaction = set_current_transaction.clone();
        let set_transaction_items = set_transaction_items.clone();
        let set_change_amount = set_change_amount.clone();
        let set_open_transactions = set_open_transactions.clone();
        
        leptos::task::spawn_local(async move {
            let customer_name = if name.is_empty() { None } else { Some(name) };
            
            if let Ok(transaction) = create_transaction(customer_name).await {
                set_current_transaction.set(Some(transaction.id));
                set_transaction_items.set(vec![]);
                set_change_amount.set(None);
                
                // Refresh open transactions
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
                set_customer_name.set(details.transaction.customer_name.unwrap_or_default());
                set_show_open_transactions.set(false);
            }
        });
    };

    let update_transaction = move |_| {
        let current_trans = current_transaction.get();
        let name = customer_name.get();
        let customer_name = if name.is_empty() { None } else { Some(name) };

        if let Some(trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                if update_transaction(trans_id, customer_name).await.is_ok() {
                }
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
                if remove_item_from_transaction(trans_id, item_id).await.is_ok() {
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
        let fetch_last_closed_transaction = fetch_last_closed_transaction.clone();

        if let Some(trans_id) = current_trans {
            if let Ok(amount) = amount_str.parse::<f64>() {
                leptos::task::spawn_local(async move {
                    if let Ok(response) = close_transaction(trans_id, amount).await {
                        set_change_amount.set(Some(response.change_amount));
                        set_current_transaction.set(None);
                        set_customer_name.set(String::new());
                        
                        // Refresh open transactions
                        if let Ok(trans) = fetch_open_transactions().await {
                            set_open_transactions.set(trans);
                        }
                        // Refresh last closed transaction
                        fetch_last_closed_transaction();
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
        let fetch_last_closed_transaction = fetch_last_closed_transaction.clone();

        if let Some(trans_id) = current_trans {
            leptos::task::spawn_local(async move {
                if cancel_transaction(trans_id).await.is_ok() {
                    set_current_transaction.set(None);
                    set_transaction_items.set(vec![]);
                    set_customer_name.set(String::new());
                    
                    // Refresh open transactions
                    if let Ok(trans) = fetch_open_transactions().await {
                        set_open_transactions.set(trans);
                    }
                    // Refresh last closed transaction
                    fetch_last_closed_transaction();
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
                // Refresh open transactions
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
                                
                                // Show button to resume a transaction if any are open
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

                                // Show the last change amount if available
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

                                // Display a list of all the open transactions
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
                                                <strong>
                                                    "Customer: "
                                                </strong>
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
                                                <button class="btn-primary-small" on:click=update_transaction>
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
                                <button
                                    class="action-button pause" on:click=pause_sale>
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
                    let dto = CreateItemDto {
                        name: name.get(),
                        description: Some(description.get()).filter(|s| !s.is_empty()),
                        price: price_val,
                        category_id: cat_id,
                        sku: Some(sku.get()).filter(|s| !s.is_empty()),
                        in_stock: Some(in_stock.get()),
                    };
                    
                    leptos::task::spawn_local(async move {
                        if create_item(dto).await.is_ok() {
                            load_data();
                            set_creating_item.set(false);
                        }
                    });
                } else if let Some(item) = editing {
                    let dto = UpdateItemDto {
                        name: Some(name.get()),
                        description: Some(description.get()).filter(|s| !s.is_empty()),
                        price: Some(price_val),
                        category_id: Some(cat_id),
                        sku: Some(sku.get()).filter(|s| !s.is_empty()),
                        in_stock: Some(in_stock.get()),
                    };
                    
                    leptos::task::spawn_local(async move {
                        if update_item(item.id, dto).await.is_ok() {
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
    let (deleting_category, set_deleting_category) = signal(Option::<(Uuid, String)>::None);
    
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
            let dto = CreateCategoryDto {
                name: name.get(),
                description: Some(description.get()).filter(|s| !s.is_empty()),
            };
            
            leptos::task::spawn_local(async move {
                if create_category(dto).await.is_ok() {
                    load_categories();
                    set_creating_category.set(false);
                }
            });
        } else if let Some(category) = editing {
            let dto = UpdateCategoryDto {
                name: Some(name.get()),
                description: Some(description.get()).filter(|s| !s.is_empty()),
            };
            
            leptos::task::spawn_local(async move {
                if update_category(category.id, dto).await.is_ok() {
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
    
    // Initialize dates to reasonable defaults
    Effect::new(move || {
        let today = Utc::now();
        let week_ago = today - chrono::Duration::days(7);
        set_end_date.set(today.format("%Y-%m-%d").to_string());
        set_start_date.set(week_ago.format("%Y-%m-%d").to_string());
    });
    
    let load_report = move |report_type: String| {
        set_loading.set(true);
        set_error.set(None);
        
        leptos::task::spawn_local(async move {
            let result = match report_type.as_str() {
                "daily" => fetch_daily_report().await,
                "monthly" => fetch_monthly_report().await,
                "custom" => {
                    if let (Ok(start), Ok(end)) = (
                        start_date.get().parse::<chrono::NaiveDate>(),
                        end_date.get().parse::<chrono::NaiveDate>()
                    ) {
                        let start_dt = start.and_hms_opt(0, 0, 0)
                            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                        let end_dt = end.and_hms_opt(23, 59, 59)
                            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                        
                        if let (Some(start_dt), Some(end_dt)) = (start_dt, end_dt) {
                            fetch_sales_report(start_dt, end_dt).await
                        } else {
                            Err("Invalid date format".to_string())
                        }
                    } else {
                        Err("Please select valid start and end dates".to_string())
                    }
                },
                _ => Err("Invalid report type".to_string()),
            };
            
            match result {
                Ok(report_data) => {
                    set_report.set(Some(report_data));
                    set_error.set(None);
                },
                Err(e) => {
                    set_error.set(Some(e));
                    set_report.set(None);
                },
            }
            set_loading.set(false);
        });
    };
    
    // Load daily report on mount
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

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App)
}