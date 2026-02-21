use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse, http::StatusCode,
};
use futures::future::LocalBoxFuture;
use parking_lot::RwLock;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use std::{collections::HashMap, future::Ready};
use log::warn;

/// Rate limiting middleware that tracks requests per IP
pub struct RateLimitMiddleware {
    /// Maximum requests allowed per time window
    max_requests: u32,
    /// Time window for rate limiting
    time_window: Duration,
}

impl RateLimitMiddleware {
    pub fn new(max_requests: u32, time_window: Duration) -> Self {
        RateLimitMiddleware {
            max_requests,
            time_window,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RateLimitMiddlewareService {
            service,
            max_requests: self.max_requests,
            time_window: self.time_window,
            requests: RwLock::new(HashMap::new()),
        }))
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: S,
    max_requests: u32,
    time_window: Duration,
    requests: RwLock<HashMap<String, Vec<Instant>>>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract the client IP address
        let ip = req
            .peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let max_requests = self.max_requests;
        let time_window = self.time_window;
        let requests = self.requests.clone();

        let now = Instant::now();
        let mut request_map = requests.write();

        // Clean up old requests and check rate limit
        let request_times = request_map
            .entry(ip.clone())
            .or_insert_with(Vec::new);

        // Remove requests outside the time window
        request_times.retain(|&req_time| now.duration_since(req_time) < time_window);

        if request_times.len() >= max_requests as usize {
            warn!("Rate limit exceeded for IP: {}", ip);
            return Box::pin(async move {
                Ok(req.error_response(
                    HttpResponse::TooManyRequests()
                        .json(serde_json::json!({
                            "error": "Rate limit exceeded",
                            "message": "Too many requests. Please try again later.",
                            "retry_after": time_window.as_secs()
                        }))
                        .into(),
                ))
            });
        }

        request_times.push(now);
        drop(request_map);

        let fut = self.service.call(req);

        Box::pin(async move { fut.await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_creation() {
        let limiter = RateLimitMiddleware::new(100, Duration::from_secs(60));
        assert_eq!(limiter.max_requests, 100);
    }
}
