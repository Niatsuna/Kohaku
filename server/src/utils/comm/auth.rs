/*
  This file holds the complete authentication logic.
  For websocket logic look at ws.rs
*/

use chrono::Utc;
use hex;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::utils::comm::WsMessage;

type HmacSha256 = Hmac<Sha256>;

/// Checks if given message is parseable, has a valid signature and is not expired.
/// Returns either the parsed message or an error.
pub fn verify_message(data: &str, secret: &[u8]) -> Result<WsMessage, String> {
    let parts: Vec<&str> = data.split('.').collect();
    if parts.len() != 2 {
        return Err("Invalid message format".into());
    }

    let payload = parts[0];
    let signature = parts[1];

    // Verify signature
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| "Invalid secret")?;
    mac.update(payload.as_bytes());

    let expected_sig = hex::encode(mac.finalize().into_bytes());
    if expected_sig != signature {
        return Err("Invalid signature".into());
    }

    // Check timestamp
    let message: WsMessage =
        serde_json::from_str(payload).map_err(|e| format!("Invalid JSON: {}", e))?;
    let now = Utc::now().timestamp();
    if (now - message.timestamp).abs() > 30 {
        return Err("Message expired".into());
    }

    Ok(message)
}

/// Signs message to fit HMAC style for further communication
pub fn sign_message(message: &WsMessage, secret: &[u8]) -> String {
    let payload = serde_json::to_string(message).unwrap();

    let mut mac = HmacSha256::new_from_slice(secret).unwrap();
    mac.update(payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    format!("{}.{}", payload, signature)
}
