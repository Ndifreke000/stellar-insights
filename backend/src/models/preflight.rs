use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Machine-readable reason code attached to a preflight check.
///
/// Integrators can branch on these stable codes instead of parsing the
/// free-text `detail` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreflightCheckCode {
    /// Historical success rate for the corridor is below the threshold.
    LowSuccessRate,
    /// Corridor liquidity is too thin for the requested amount.
    ThinLiquidity,
    /// Not enough historical samples to make a confident decision.
    SmallSample,
    /// The corridor (asset pair) is not recognized.
    UnknownCorridor,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreflightCheck {
    /// Stable machine-readable reason code for this check.
    pub code: PreflightCheckCode,
    /// Human-readable explanation of the check outcome.
    pub detail: String,
    /// Whether this check passed.
    pub passed: bool,
}

impl PreflightCheck {
    pub fn new(code: PreflightCheckCode, passed: bool, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            passed,
        }
    }
}
