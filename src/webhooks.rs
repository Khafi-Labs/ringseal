use crate::{errors::AppError, models::WebhookPayload};
use reqwest::{Client, redirect::Policy};
use std::time::Duration;
use url::Url;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

pub fn validate_webhook_url(url_str: &str, require_https: bool) -> Result<(), AppError> {
    let url = Url::parse(url_str).map_err(|_| AppError::BadRequest("Invalid webhook URL".into()))?;

    if require_https && url.scheme() != "https" {
        return Err(AppError::BadRequest("Webhook URL must use HTTPS".into()));
    }

    if let Some(host) = url.host_str() {
        let host_lower = host.to_lowercase();
        let private_patterns = [
            "localhost",
            "127.",
            "10.",
            "172.16.", "172.17.", "172.18.", "172.19.", "172.20.", "172.21.", "172.22.", "172.23.", "172.24.", "172.25.", "172.26.", "172.27.", "172.28.", "172.29.", "172.30.", "172.31.",
            "192.168.",
            "169.254.",
            "[::1]",
            "[fc",
            "[fe",
        ];
        
        for pattern in private_patterns {
            if host_lower.starts_with(pattern) || host_lower == pattern {
                return Err(AppError::BadRequest("Webhook URL cannot point to private network addresses".into()));
            }
        }
    } else {
        return Err(AppError::BadRequest("Webhook URL must have a valid host".into()));
    }

    Ok(())
}

pub async fn dispatch_webhook(url: &str, payload: &WebhookPayload, hmac_secret: &str) -> Result<(), AppError> {
    // Basic validation before dispatch
    validate_webhook_url(url, true)?;

    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| AppError::Internal("Failed to build HTTP client".into()))?;

    let body = serde_json::to_string(payload).map_err(|_| AppError::Internal("Failed to serialize webhook payload".into()))?;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(hmac_secret.as_bytes())
        .map_err(|_| AppError::Internal("Invalid HMAC secret".into()))?;
    mac.update(body.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let mut backoff = 1;
    let mut attempts = 0;
    let max_attempts = 3;

    while attempts < max_attempts {
        let req = client.post(url)
            .header("X-Ringseal-Event-Id", payload.event_id.to_string())
            .header("X-Ringseal-Timestamp", payload.timestamp.to_rfc3339())
            .header("X-Ringseal-Signature", &signature)
            .header("Content-Type", "application/json")
            .body(body.clone());

        match req.send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => {
                attempts += 1;
                if attempts == max_attempts {
                    let parsed = Url::parse(url).unwrap();
                    tracing::error!("Webhook delivery failed after 3 attempts to {}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
                    return Err(AppError::Internal("Webhook delivery failed".into()));
                }
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff *= 2;
            }
        }
    }

    Ok(())
}

pub fn spawn_webhook(url: String, payload: WebhookPayload, hmac_secret: String) {
    tokio::spawn(async move {
        if let Err(e) = dispatch_webhook(&url, &payload, &hmac_secret).await {
            tracing::warn!("Spawned webhook failed: {:?}", e);
        }
    });
}
