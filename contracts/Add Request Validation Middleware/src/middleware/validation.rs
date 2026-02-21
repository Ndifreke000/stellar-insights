use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use log::{debug, info};
use std::future::{ready, Ready};

/// Validation middleware for comprehensive input validation
pub struct ValidationMiddleware;

impl<S, B> Transform<S, ServiceRequest> for ValidationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ValidationMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ValidationMiddlewareService { service }))
    }
}

pub struct ValidationMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ValidationMiddlewareService<S>
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
        debug!("Validation middleware processing request: {} {}", req.method(), req.path());

        // Extract and validate request metadata
        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/json")
            .to_string();

        // Check if content-type is valid for POST/PUT/PATCH requests
        match req.method() {
            actix_web::http::Method::POST
            | actix_web::http::Method::PUT
            | actix_web::http::Method::PATCH => {
                if !is_valid_content_type(&content_type) {
                    info!("Invalid content-type: {}", content_type);
                    return Box::pin(async move {
                        Err(crate::errors::ValidationError::InvalidContentType(content_type).into())
                    });
                }
            }
            _ => {}
        }

        let fut = self.service.call(req);

        Box::pin(async move {
            debug!("Processing request through validation middleware");
            let res = fut.await?;
            Ok(res)
        })
    }
}

/// Check if the content type is valid
fn is_valid_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "application/json"
            | "application/x-www-form-urlencoded"
            | "multipart/form-data"
            | ct if ct.starts_with("application/json")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_content_types() {
        assert!(is_valid_content_type("application/json"));
        assert!(is_valid_content_type("application/x-www-form-urlencoded"));
        assert!(is_valid_content_type("application/json; charset=utf-8"));
    }

    #[test]
    fn test_invalid_content_types() {
        assert!(!is_valid_content_type("text/html"));
        assert!(!is_valid_content_type("text/plain"));
    }
}
