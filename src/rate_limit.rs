use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::errors::AppError;

struct SlidingWindow {
    prev_count: u64,
    curr_count: u64,
    window_start: Instant,
    window_duration: Duration,
}

impl SlidingWindow {
    fn new(window_duration: Duration) -> Self {
        Self {
            prev_count: 0,
            curr_count: 0,
            window_start: Instant::now(),
            window_duration,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_start);

        if elapsed >= self.window_duration {
            let cycles = (elapsed.as_nanos() / self.window_duration.as_nanos()) as u64;
            if cycles == 1 {
                self.prev_count = self.curr_count;
            } else {
                self.prev_count = 0;
            }
            self.curr_count = 0;
            self.window_start = now; // In a precise implementation, we'd add the duration, but resetting to now is fine for rate limiting
        }
    }

    fn check_and_add(&mut self, max_requests: u64) -> bool {
        self.update();

        let now = Instant::now();
        let elapsed = now.duration_since(self.window_start);
        
        let elapsed_fraction = elapsed.as_secs_f64() / self.window_duration.as_secs_f64();
        let weight = (1.0 - elapsed_fraction).max(0.0);
        
        let estimated_prev = (self.prev_count as f64 * weight) as u64;
        let effective_count = estimated_prev + self.curr_count;

        if effective_count >= max_requests {
            return false;
        }

        self.curr_count += 1;
        true
    }
}

pub struct RateLimiter {
    limits: Mutex<HashMap<String, SlidingWindow>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_rate_limit(&self, key: &str, max_requests: u64, window_seconds: u64) -> Result<(), AppError> {
        let mut limits = self.limits.lock().map_err(|_| AppError::Internal("Rate limiter lock poisoned".into()))?;
        
        let window = limits.entry(key.to_string()).or_insert_with(|| SlidingWindow::new(Duration::from_secs(window_seconds)));
        
        if window.window_duration != Duration::from_secs(window_seconds) {
            window.window_duration = Duration::from_secs(window_seconds);
        }

        if !window.check_and_add(max_requests) {
            return Err(AppError::RateLimited);
        }
        
        Ok(())
    }

    pub fn check_ip_rate_limit(&self, ip: &str, max_requests: u64, window_seconds: u64) -> Result<(), AppError> {
        let key = format!("ip:{}", ip);
        self.check_rate_limit(&key, max_requests, window_seconds)
    }

    pub fn cleanup(&self, window_seconds: u64) {
        if let Ok(mut limits) = self.limits.lock() {
            let now = Instant::now();
            let duration = Duration::from_secs(window_seconds);
            limits.retain(|_, window| {
                now.duration_since(window.window_start) <= duration * 2
            });
        }
    }
}
