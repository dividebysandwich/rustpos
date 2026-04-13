use futures_util::{SinkExt, StreamExt};
use rustpos_common::printer::{find_printer, print_receipt};
use rustpos_common::protocol::*;
use tokio_tungstenite::tungstenite::Message;

#[derive(serde::Deserialize)]
struct Config {
    server_url: String,
    passphrase: String,
    /// Override the logo downloaded from the server with a local file
    logo_path: Option<String>,
    reconnect_delay_secs: Option<u64>,
}

#[tokio::main]
async fn main() {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "printclient.toml".into());
    let config_str =
        std::fs::read_to_string(&config_path).expect("Failed to read config file");
    let config: Config =
        toml::from_str(&config_str).expect("Failed to parse config file");

    let reconnect_delay = config.reconnect_delay_secs.unwrap_or(5);
    let ws_url = format!(
        "{}/ws/printer",
        config.server_url.trim_end_matches('/')
    );

    println!("RustPOS Print Client");
    println!("Server: {}", ws_url);

    // Check for printer at startup
    match find_printer() {
        Ok((path, _)) => println!("Found printer at: {}", path),
        Err(e) => eprintln!("Warning: No printer found at startup: {}", e),
    }

    loop {
        println!("Connecting to {}...", ws_url);
        match connect_and_run(&ws_url, &config.passphrase, config.logo_path.as_deref()).await {
            Ok(()) => println!("Connection closed"),
            Err(e) => eprintln!("Connection error: {}", e),
        }
        println!("Reconnecting in {} seconds...", reconnect_delay);
        tokio::time::sleep(std::time::Duration::from_secs(reconnect_delay)).await;
    }
}

async fn connect_and_run(
    url: &str,
    passphrase: &str,
    logo_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (ws, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = ws.split();

    // Send auth
    let auth = serde_json::to_string(&ClientMessage::Auth {
        passphrase: passphrase.to_string(),
    })?;
    write.send(Message::Text(auth.into())).await?;

    // Wait for auth response
    let resp = read
        .next()
        .await
        .ok_or("Connection closed before auth response")??;

    if let Message::Text(text) = resp {
        match serde_json::from_str::<ServerMessage>(&text)? {
            ServerMessage::AuthOk => println!("Authenticated successfully"),
            ServerMessage::AuthFail { reason } => {
                return Err(format!("Authentication failed: {}", reason).into());
            }
            _ => return Err("Unexpected message during auth".into()),
        }
    } else {
        return Err("Expected text message for auth response".into());
    }

    // Receive logo from server (or use local override)
    let logo_path = if let Some(path) = logo_override {
        println!("Using local logo override: {}", path);
        Some(path.to_string())
    } else {
        receive_logo(&mut read).await?
    };

    if logo_path.is_some() {
        println!("Logo ready for printing");
    } else {
        println!("No logo available, receipts will print without logo");
    }

    // Main receive loop
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ServerMessage>(&text) {
                Ok(ServerMessage::PrintReceipt(job)) => {
                    println!(
                        "Received print job: {} items, total {:.2}",
                        job.items.len(),
                        job.items
                            .iter()
                            .map(|(_, q, p)| *q as f32 * p)
                            .sum::<f32>()
                    );
                    let logo_ref = logo_path.clone();
                    let result =
                        tokio::task::spawn_blocking(move || print_job(job, logo_ref.as_deref()))
                            .await?;

                    let response = match result {
                        Ok(()) => {
                            println!("Print complete");
                            ClientMessage::PrintOk
                        }
                        Err(e) => {
                            eprintln!("Print error: {}", e);
                            ClientMessage::PrintError {
                                message: e.to_string(),
                            }
                        }
                    };
                    write
                        .send(Message::Text(serde_json::to_string(&response)?.into()))
                        .await?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Receive the logo from the server and cache it to a local file.
/// Returns the path to the cached logo, or None if the server has no logo.
async fn receive_logo(
    read: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    use base64::Engine;

    let msg = read
        .next()
        .await
        .ok_or("Connection closed before logo message")??;

    if let Message::Text(text) = msg {
        match serde_json::from_str::<ServerMessage>(&text)? {
            ServerMessage::Logo { data: Some(b64) } => {
                let bytes = base64::engine::general_purpose::STANDARD.decode(&b64)?;
                let cache_path = "logo_receipt.png";
                std::fs::write(cache_path, &bytes)?;
                println!("Downloaded logo from server ({} bytes)", bytes.len());
                Ok(Some(cache_path.to_string()))
            }
            ServerMessage::Logo { data: None } => {
                println!("Server has no receipt logo configured");
                Ok(None)
            }
            _ => Err("Expected logo message after auth".into()),
        }
    } else {
        Err("Expected text message for logo".into())
    }
}

fn print_job(
    job: PrintReceiptJob,
    logo_path: Option<&str>,
) -> Result<(), String> {
    let (path, mut printer) = find_printer().map_err(|e| e.to_string())?;
    println!("Printing on {}", path);

    let datetime = chrono::NaiveDateTime::parse_from_str(&job.datetime, "%Y-%m-%d %H:%M:%S")
        .map(|dt| {
            dt.and_local_timezone(chrono::Local)
                .single()
                .unwrap_or_else(chrono::Local::now)
        })
        .unwrap_or_else(|_| chrono::Local::now());

    print_receipt(
        &mut printer,
        job.items,
        job.paid_amount,
        job.change,
        datetime,
        logo_path,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
