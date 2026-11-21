use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::error::ApiError;

/// Default fields that may never be requested or exposed via field selection.
pub const DEFAULT_FORBIDDEN_FIELDS: &[&str] = &[
    "password",
    "password_hash",
    "key_hash",
    "secret",
    "encrypted_secret",
    "totp_secret",
    "private_key",
    "signing_key",
    "jwt_secret",
    "refresh_token_hash",
    "api_key_hash",
    "salt",
];

/// Default fields restricted to authenticated admin callers (`AuthUser.is_admin == true`).
pub const DEFAULT_ADMIN_ONLY_FIELDS: &[&str] = &[
    "added_by_user_id",
    "ip_address",
    "session_id",
    "stack_trace",
    "audit_hash",
    "device_user_agent",
];

/// Envelope keys preserved when applying field projection to paginated/list payloads.
const ENVELOPE_COLLECTION_KEYS: &[&str] = &["data", "items", "entries", "results"];
const ENVELOPE_META_KEYS: &[&str] = &[
    "pagination",
    "links",
    "total",
    "page",
    "page_size",
    "total_pages",
    "next_cursor",
    "count",
    "corridor_key",
];

/// Role-aware field authorization policy applied to both incoming `fields`
/// parameters and outgoing JSON responses (#2832).
#[derive(Debug, Clone)]
pub struct FieldAccessPolicy {
    pub always_forbidden: HashSet<String>,
    pub admin_only: HashSet<String>,
}

impl Default for FieldAccessPolicy {
    fn default() -> Self {
        Self {
            always_forbidden: DEFAULT_FORBIDDEN_FIELDS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            admin_only: DEFAULT_ADMIN_ONLY_FIELDS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }
}

impl FieldAccessPolicy {
    /// Returns `true` when `field` is forbidden regardless of caller role.
    #[must_use]
    pub fn is_always_forbidden(&self, field: &str) -> bool {
        self.always_forbidden.contains(&field.to_ascii_lowercase())
    }

    /// Returns `true` when `field` requires admin privileges.
    #[must_use]
    pub fn is_admin_only(&self, field: &str) -> bool {
        self.admin_only.contains(&field.to_ascii_lowercase())
    }

    /// Validate that every requested field in `selection` is authorized for the caller.
    pub fn authorize(
        &self,
        selection: &FieldSelection,
        is_admin: bool,
    ) -> Result<(), FieldSelectionError> {
        let mut sorted: Vec<&String> = selection.fields.iter().collect();
        sorted.sort();
        for field in sorted {
            if self.is_always_forbidden(field) {
                return Err(FieldSelectionError::ForbiddenField(field.clone()));
            }
            if !is_admin && self.is_admin_only(field) {
                return Err(FieldSelectionError::AdminFieldRequired(field.clone()));
            }
        }
        Ok(())
    }

    /// Filter a 2xx JSON response payload according to the caller's role and
    /// optional [`FieldSelection`]. Sensitive fields are always stripped,
    /// admin-only fields are stripped for non-admin callers, and non-requested
    /// fields are omitted when a selection is provided.
    #[must_use]
    pub fn filter_json(
        &self,
        value: &serde_json::Value,
        selection: Option<&FieldSelection>,
        is_admin: bool,
    ) -> serde_json::Value {
        let active_selection = selection.filter(|s| !s.fields.is_empty());
        match value {
            serde_json::Value::Object(map) => {
                // If the top-level object is a list envelope (`data` / `items` /
                // `entries` / `results`), preserve pagination metadata keys and
                // project each element inside the collection.
                let is_envelope = ENVELOPE_COLLECTION_KEYS
                    .iter()
                    .any(|k| map.contains_key(*k));
                if is_envelope {
                    let mut out = serde_json::Map::with_capacity(map.len());
                    for (k, v) in map {
                        if self.is_disallowed_key(k, is_admin) {
                            continue;
                        }
                        if ENVELOPE_COLLECTION_KEYS.contains(&k.as_str()) {
                            out.insert(
                                k.clone(),
                                self.filter_entity_or_array(v, active_selection, is_admin),
                            );
                        } else if ENVELOPE_META_KEYS.contains(&k.as_str())
                            || active_selection.is_none()
                            || active_selection.is_some_and(|sel| sel.includes(k))
                        {
                            out.insert(k.clone(), self.redact_sensitive(v, is_admin));
                        }
                    }
                    serde_json::Value::Object(out)
                } else {
                    self.filter_object(map, active_selection, is_admin)
                }
            }
            serde_json::Value::Array(items) => serde_json::Value::Array(
                items
                    .iter()
                    .map(|item| self.filter_entity_or_array(item, active_selection, is_admin))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn filter_entity_or_array(
        &self,
        value: &serde_json::Value,
        selection: Option<&FieldSelection>,
        is_admin: bool,
    ) -> serde_json::Value {
        match value {
            serde_json::Value::Array(items) => serde_json::Value::Array(
                items
                    .iter()
                    .map(|item| self.filter_entity_or_array(item, selection, is_admin))
                    .collect(),
            ),
            serde_json::Value::Object(map) => self.filter_object(map, selection, is_admin),
            other => other.clone(),
        }
    }

    fn filter_object(
        &self,
        map: &serde_json::Map<String, serde_json::Value>,
        selection: Option<&FieldSelection>,
        is_admin: bool,
    ) -> serde_json::Value {
        let mut out = serde_json::Map::with_capacity(map.len());
        for (k, v) in map {
            if self.is_disallowed_key(k, is_admin) {
                continue;
            }
            if selection.is_none_or(|sel| sel.includes(k)) {
                out.insert(k.clone(), self.redact_sensitive(v, is_admin));
            }
        }
        serde_json::Value::Object(out)
    }

    fn redact_sensitive(&self, value: &serde_json::Value, is_admin: bool) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut out = serde_json::Map::with_capacity(map.len());
                for (k, v) in map {
                    if self.is_disallowed_key(k, is_admin) {
                        continue;
                    }
                    out.insert(k.clone(), self.redact_sensitive(v, is_admin));
                }
                serde_json::Value::Object(out)
            }
            serde_json::Value::Array(items) => serde_json::Value::Array(
                items
                    .iter()
                    .map(|item| self.redact_sensitive(item, is_admin))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn is_disallowed_key(&self, key: &str, is_admin: bool) -> bool {
        self.is_always_forbidden(key) || (!is_admin && self.is_admin_only(key))
    }
}

/// Configuration for the field selection middleware.
#[derive(Debug, Clone)]
pub struct FieldSelectionConfig {
    /// Maximum number of fields that may be requested in a single query.
    pub max_fields: usize,
    /// Role-aware field authorization policy.
    pub access_policy: FieldAccessPolicy,
}

impl Default for FieldSelectionConfig {
    fn default() -> Self {
        Self {
            max_fields: 50,
            access_policy: FieldAccessPolicy::default(),
        }
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
            if !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
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

/// Errors that can occur while parsing or authorizing a field selection parameter.
#[derive(Debug, thiserror::Error)]
pub enum FieldSelectionError {
    #[error("field list must not be empty")]
    EmptyFieldList,
    #[error("too many fields requested: {requested} (max {max})")]
    TooManyFields { requested: usize, max: usize },
    #[error("invalid field name '{0}': only alphanumeric characters and underscores are allowed")]
    InvalidFieldName(String),
    #[error("field '{0}' is forbidden and cannot be selected")]
    ForbiddenField(String),
    #[error("field '{0}' requires admin privileges")]
    AdminFieldRequired(String),
}

impl FieldSelectionError {
    /// Map to the project's standard [`ApiError`] shape.
    #[must_use]
    pub fn to_api_error(&self) -> ApiError {
        match self {
            Self::EmptyFieldList
            | Self::TooManyFields { .. }
            | Self::InvalidFieldName(_) => {
                ApiError::bad_request("INVALID_FIELD_SELECTION", self.to_string())
            }
            Self::ForbiddenField(_) | Self::AdminFieldRequired(_) => {
                ApiError::forbidden("FORBIDDEN_FIELD_SELECTION", self.to_string())
            }
        }
    }
}
