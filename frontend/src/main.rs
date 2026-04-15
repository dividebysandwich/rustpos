#![recursion_limit = "512"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::config::get_configuration;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use rustpos::app::{shell, App};
    use rustpos::printer::find_printer;
    use sqlx::sqlite::SqlitePool;
    use std::env;
    use std::net::SocketAddr;
    use tokio::sync::broadcast;

    tracing_subscriber::fmt::init();

    // Look for POS printer on any serial or USB port
    println!("Searching for POS printer...");
    match find_printer() {
        Ok((path, _printer)) => {
            println!("Found printer at: {}", path);
        }
        Err(e) => {
            eprintln!("Error finding printer: {}", e);
        }
    }

    // Create data directories if they don't exist
    std::fs::create_dir_all("data/item_images").ok();

    // Create database connection with auto-create
    let db = SqlitePool::connect("sqlite:data/pos.db?mode=rwc")
        .await
        .expect("Failed to connect to database");

    // Run migrations inline
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS categories (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"#,
    )
    .execute(&db)
    .await
    .expect("Failed to create categories table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS items (
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
        )"#,
    )
    .execute(&db)
    .await
    .expect("Failed to create items table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS transactions (
            id TEXT PRIMARY KEY,
            customer_name TEXT,
            status TEXT NOT NULL CHECK (status IN ('open', 'closed', 'cancelled')),
            total REAL NOT NULL DEFAULT 0,
            paid_amount REAL,
            change_amount REAL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT
        )"#,
    )
    .execute(&db)
    .await
    .expect("Failed to create transactions table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS transaction_items (
            id TEXT PRIMARY KEY,
            transaction_id TEXT NOT NULL,
            item_id TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            unit_price REAL NOT NULL,
            total_price REAL NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE,
            FOREIGN KEY (item_id) REFERENCES items(id)
        )"#,
    )
    .execute(&db)
    .await
    .expect("Failed to create transaction_items table");

    // Migrations for new columns
    sqlx::query("ALTER TABLE items ADD COLUMN image_path TEXT").execute(&db).await.ok();
    sqlx::query("ALTER TABLE items ADD COLUMN stock_quantity INTEGER").execute(&db).await.ok();
    sqlx::query("ALTER TABLE items ADD COLUMN kitchen_item BOOLEAN NOT NULL DEFAULT 0").execute(&db).await.ok();
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS kitchen_order_items (
            id TEXT PRIMARY KEY,
            transaction_id TEXT NOT NULL,
            transaction_item_id TEXT NOT NULL,
            item_id TEXT NOT NULL,
            item_name TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            customer_name TEXT,
            completed BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE
        )"#,
    ).execute(&db).await.ok();

    // User accounts and sessions
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            pin_hash TEXT NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('admin', 'cashier', 'cook')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"#,
    )
    .execute(&db)
    .await
    .expect("Failed to create users table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            token TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
    )
    .execute(&db)
    .await
    .expect("Failed to create sessions table");

    // Configuration table
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )"#,
    )
    .execute(&db)
    .await
    .expect("Failed to create config table");

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_category_id ON items(category_id)")
        .execute(&db)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_transaction_items_transaction_id ON transaction_items(transaction_id)")
        .execute(&db)
        .await
        .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_transaction_items_item_id ON transaction_items(item_id)",
    )
    .execute(&db)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status)",
    )
    .execute(&db)
    .await
    .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_transactions_customer_name ON transactions(customer_name)",
    )
    .execute(&db)
    .await
    .ok();

    println!("Database initialized successfully!");

    let conf = get_configuration(None).expect("Failed to get Leptos configuration");
    let mut leptos_options = conf.leptos_options;
    // Ensure site_root is relative to cwd so the binary is portable
    leptos_options.site_root = "site".into();
    let routes = generate_route_list(App);

    let (kitchen_tx, _) = broadcast::channel::<()>(16);
    let (printer_tx, _) = broadcast::channel::<rustpos_common::protocol::PrintReceiptJob>(16);
    let (display_tx, _) = broadcast::channel::<String>(16);

    let app = Router::new()
        .route("/ws/kitchen", axum::routing::get(kitchen_ws_handler))
        .route("/ws/printer", axum::routing::get(printer_ws_handler))
        .route("/ws/display", axum::routing::get(display_ws_handler))
        .nest_service("/item_images", tower_http::services::ServeDir::new("data/item_images"))
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let db = db.clone();
                let kitchen_tx = kitchen_tx.clone();
                let printer_tx = printer_tx.clone();
                let display_tx = display_tx.clone();
                move || {
                    provide_context(db.clone());
                    provide_context(kitchen_tx.clone());
                    provide_context(printer_tx.clone());
                    provide_context(display_tx.clone());
                }
            },
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(axum::Extension(kitchen_tx))
        .layer(axum::Extension(printer_tx))
        .layer(axum::Extension(display_tx))
        .layer(axum::Extension(db.clone()))
        .with_state(leptos_options);

    let port = env::var("RUSTPOS_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("RustPOS is accessible on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "ssr")]
async fn kitchen_ws_handler(
    wsu: axum::extract::ws::WebSocketUpgrade,
    axum::Extension(tx): axum::Extension<tokio::sync::broadcast::Sender<()>>,
) -> impl axum::response::IntoResponse {
    use axum::extract::ws::Message;
    wsu.on_upgrade(move |mut socket| async move {
        let mut rx = tx.subscribe();
        let _ = socket.send(Message::Text("refresh".into())).await;
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(()) => {
                            if socket.send(Message::Text("refresh".into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(_)) => {}
                        _ => break,
                    }
                }
            }
        }
    })
}

#[cfg(feature = "ssr")]
async fn display_ws_handler(
    wsu: axum::extract::ws::WebSocketUpgrade,
    axum::Extension(tx): axum::Extension<tokio::sync::broadcast::Sender<String>>,
) -> impl axum::response::IntoResponse {
    use axum::extract::ws::Message;
    wsu.on_upgrade(move |mut socket| async move {
        let mut rx = tx.subscribe();
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if socket.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(_)) => {}
                        _ => break,
                    }
                }
            }
        }
    })
}

#[cfg(feature = "ssr")]
async fn validate_printer_passphrase(db: &sqlx::SqlitePool, passphrase: &str) -> bool {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let stored: Option<String> =
        sqlx::query_scalar("SELECT value FROM config WHERE key = 'printer_passphrase'")
            .fetch_optional(db)
            .await
            .unwrap_or(None);

    stored.is_some_and(|s| s == hash)
}

#[cfg(feature = "ssr")]
async fn printer_ws_handler(
    wsu: axum::extract::ws::WebSocketUpgrade,
    axum::Extension(tx): axum::Extension<
        tokio::sync::broadcast::Sender<rustpos_common::protocol::PrintReceiptJob>,
    >,
    axum::Extension(db): axum::Extension<sqlx::sqlite::SqlitePool>,
) -> impl axum::response::IntoResponse {
    use axum::extract::ws::Message;
    use rustpos_common::protocol::*;

    wsu.on_upgrade(move |mut socket| async move {
        // Phase 1: Authentication (10 second timeout)
        let auth_timeout = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            socket.recv(),
        )
        .await;

        let authenticated = match auth_timeout {
            Ok(Some(Ok(Message::Text(text)))) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Auth { passphrase }) => {
                        validate_printer_passphrase(&db, &passphrase).await
                    }
                    _ => false,
                }
            }
            _ => false,
        };

        if !authenticated {
            let msg = serde_json::to_string(&ServerMessage::AuthFail {
                reason: "Invalid passphrase".into(),
            })
            .unwrap();
            let _ = socket.send(Message::Text(msg.into())).await;
            return;
        }

        let ok = serde_json::to_string(&ServerMessage::AuthOk).unwrap();
        if socket.send(Message::Text(ok.into())).await.is_err() {
            return;
        }

        // Send receipt logo to client
        let logo_data = std::fs::read("data/logo_receipt.png")
            .ok()
            .map(|bytes| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes));
        let logo_msg = serde_json::to_string(&ServerMessage::Logo { data: logo_data }).unwrap();
        if socket.send(Message::Text(logo_msg.into())).await.is_err() {
            return;
        }

        println!("Remote printer client connected and authenticated");

        // Phase 2: Print job relay loop
        let mut rx = tx.subscribe();
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(job) => {
                            let msg = serde_json::to_string(
                                &ServerMessage::PrintReceipt(job)
                            ).unwrap();
                            if socket.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                                match client_msg {
                                    ClientMessage::PrintOk => {}
                                    ClientMessage::PrintError { message } => {
                                        eprintln!("Remote printer error: {}", message);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Ok(_)) => {}
                        _ => break,
                    }
                }
            }
        }

        println!("Remote printer client disconnected");
    })
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // Required by cargo-leptos for the WASM build
}
