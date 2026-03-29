// Shared validation utilities

use validator::Validate;

pub trait Validatable {
    fn validate(&self) -> Result<(), validator::ValidationErrors>;
}

#[macro_export]
macro_rules! validate {
    ($expr:expr) => {
        $expr.validate().map_err(|e| {
            crate::error::ApiError::bad_request("VALIDATION_ERROR", e.to_string())
        })
    };
}

