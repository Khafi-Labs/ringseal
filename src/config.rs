use dotenvy::dotenv;
use std::env;

const KNOWN_WEAK_SECRETS: &[&str] = &[
    "dev-master-secret-change-in-production",
    "dev-hmac-secret-change-in-production",
    "dev-jwt-secret-change-in-production",
    "dev-admin-secret",
    "secret",
    "changeme",
    "password",
];

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub internal_port: u16,
    pub code_expiry_seconds: i64,
    pub max_attempts: i32,
    pub max_sessions_per_customer: i32,
    pub max_sessions_window_seconds: i64,
    pub rate_limit_requests: u64,
    pub rate_limit_window_seconds: u64,
    pub unauth_rate_limit_requests: u64,
    pub unauth_rate_limit_window_seconds: u64,
    pub master_secret: String,
    pub admin_secret: String,
    pub trusted_proxy_count: usize,
    pub max_jwt_lifetime_seconds: i64,
    pub webhook_require_https: bool,
}

pub fn validate_secret_strength(secret: &str, name: &str) {
    if KNOWN_WEAK_SECRETS.contains(&secret) {
        panic!("{} is using a known weak default secret. This is not allowed in production.", name);
    }
    
    // Admin secret must be >= 16 chars, master secret >= 32 chars
    let min_len = if name == "ADMIN_SECRET" { 16 } else { 32 };
    
    if secret.len() < min_len {
        panic!("{} must be at least {} characters long.", name, min_len);
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        let _ = dotenv();
        
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("PORT must be a valid u16");
        let internal_port = env::var("INTERNAL_PORT").unwrap_or_else(|_| "9090".to_string()).parse().expect("INTERNAL_PORT must be a valid u16");
        
        let code_expiry_seconds = env::var("CODE_EXPIRY_SECONDS").unwrap_or_else(|_| "300".to_string()).parse().expect("CODE_EXPIRY_SECONDS must be a valid i64");
        let max_attempts = env::var("MAX_ATTEMPTS").unwrap_or_else(|_| "3".to_string()).parse().expect("MAX_ATTEMPTS must be a valid i32");
        let max_sessions_per_customer = env::var("MAX_SESSIONS_PER_CUSTOMER").unwrap_or_else(|_| "3".to_string()).parse().expect("MAX_SESSIONS_PER_CUSTOMER must be a valid i32");
        let max_sessions_window_seconds = env::var("MAX_SESSIONS_WINDOW_SECONDS").unwrap_or_else(|_| "300".to_string()).parse().expect("MAX_SESSIONS_WINDOW_SECONDS must be a valid i64");
        
        let rate_limit_requests = env::var("RATE_LIMIT_REQUESTS").unwrap_or_else(|_| "100".to_string()).parse().expect("RATE_LIMIT_REQUESTS must be a valid u64");
        let rate_limit_window_seconds = env::var("RATE_LIMIT_WINDOW_SECONDS").unwrap_or_else(|_| "60".to_string()).parse().expect("RATE_LIMIT_WINDOW_SECONDS must be a valid u64");
        let unauth_rate_limit_requests = env::var("UNAUTH_RATE_LIMIT_REQUESTS").unwrap_or_else(|_| "20".to_string()).parse().expect("UNAUTH_RATE_LIMIT_REQUESTS must be a valid u64");
        let unauth_rate_limit_window_seconds = env::var("UNAUTH_RATE_LIMIT_WINDOW_SECONDS").unwrap_or_else(|_| "60".to_string()).parse().expect("UNAUTH_RATE_LIMIT_WINDOW_SECONDS must be a valid u64");
        
        let master_secret = env::var("MASTER_SECRET").expect("MASTER_SECRET must be set");
        validate_secret_strength(&master_secret, "MASTER_SECRET");
        
        let admin_secret = env::var("ADMIN_SECRET").expect("ADMIN_SECRET must be set");
        validate_secret_strength(&admin_secret, "ADMIN_SECRET");
        
        let trusted_proxy_count = env::var("TRUSTED_PROXY_COUNT").unwrap_or_else(|_| "0".to_string()).parse().expect("TRUSTED_PROXY_COUNT must be a valid usize");
        let max_jwt_lifetime_seconds = env::var("MAX_JWT_LIFETIME_SECONDS").unwrap_or_else(|_| "300".to_string()).parse().expect("MAX_JWT_LIFETIME_SECONDS must be a valid i64");
        
        let webhook_require_https = env::var("WEBHOOK_REQUIRE_HTTPS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        Self {
            database_url,
            host,
            port,
            internal_port,
            code_expiry_seconds,
            max_attempts,
            max_sessions_per_customer,
            max_sessions_window_seconds,
            rate_limit_requests,
            rate_limit_window_seconds,
            unauth_rate_limit_requests,
            unauth_rate_limit_window_seconds,
            master_secret,
            admin_secret,
            trusted_proxy_count,
            max_jwt_lifetime_seconds,
            webhook_require_https,
        }
    }
}
