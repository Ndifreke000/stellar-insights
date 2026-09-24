use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for the field selection middleware.
#[derive(Debug, Clone)]
pub struct FieldSelectionConfig {
    /// Maximum number of fields that may be requested in a single query.
    pub max_fields: usize,
}

impl Default for FieldSelectionConfig {
    fn default() -> Self {
        Self { max_fields: 50 }
    }
}

/// A validated, parsed set of fields requested by the client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldSelection {
    pub fields: HashSet<String>,
}

impl FieldSelection {
    /// Parse a comma-separated `fields` query parameter value.
    ///
    /// Returns an error when the value is empty after trimming, a field name
    /// contains characters outside `[a-zA-Z0-9_]`, or the number of fields
    /// exceeds `max_fields`.
    pub fn parse(raw: &str, max_fields: usize) -> Result<Self, FieldSelectionError> {
        let fields: HashSet<String> = raw
            .split(',')
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty())
            .collect();

        if fields.is_empty() {
            return Err(FieldSelectionError::EmptyFieldList);
        }
        if fields.len() > max_fields {
            return Err(FieldSelectionError::TooManyFields {
                requested: fields.len(),
                max: max_fields,
            });
        }
        for field in &fields {
            if !field.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(FieldSelectionError::InvalidFieldName(field.clone()));
            }
        }
        Ok(Self { fields })
    }

    /// Returns `true` when the given field should be included in the response.
    /// An empty selection means "include everything".
    #[must_use]
    pub fn includes(&self, field: &str) -> bool {
        self.fields.is_empty() || self.fields.contains(field)
    }
}

/// Errors that can occur while parsing a field selection parameter.
#[derive(Debug, thiserror::Error)]
pub enum FieldSelectionError {
    #[error("field list must not be empty")]
    EmptyFieldList,
    #[error("too many fields requested: {requested} (max {max})")]
    TooManyFields { requested: usize, max: usize },
    #[error("invalid field name '{0}': only alphanumeric characters and underscores are allowed")]
    InvalidFieldName(String),
}
