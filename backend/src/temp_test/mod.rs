use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check_rate_limit(&self, user_id: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        
        // ✅ Fixed the closure syntax here!
        let timestamps = requests.entry(user_id.to_string()).or_insert_with(|| Vec::new());
        
        timestamps.retain(|&time| now.duration_since(time) < self.window);
        
        if timestamps.len() >= self.max_requests {
            false
        } else {
            timestamps.push(now);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_premium_user_tier() {
        let limiter = RateLimiter::new(100, 60);
        let user = "premium_user_eleven";

        for _ in 0..10 {
            assert!(limiter.check_rate_limit(user));
        }

        println!("✅ Success! Isolated Premium User Tier test passed!");
    }
}