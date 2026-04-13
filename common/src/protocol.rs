use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintReceiptJob {
    pub items: Vec<(String, u32, f32)>,
    pub paid_amount: f32,
    pub change: f32,
    pub datetime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "auth")]
    Auth { passphrase: String },
    #[serde(rename = "print_ok")]
    PrintOk,
    #[serde(rename = "print_error")]
    PrintError { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "auth_ok")]
    AuthOk,
    #[serde(rename = "auth_fail")]
    AuthFail { reason: String },
    #[serde(rename = "logo")]
    Logo {
        /// Base64-encoded PNG data, or None if no logo configured on server
        data: Option<String>,
    },
    #[serde(rename = "print_receipt")]
    PrintReceipt(PrintReceiptJob),
}
