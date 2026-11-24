use crate::data_quality::*;
use serde_json::json;

#[test]
fn test_not_null_expectation_passes() {
    let data = vec![json!({"asset_id": "USDC"}), json!({"asset_id": "XLM"})];
    let exp = NotNullExpectation {
        field: "asset_id".to_string(),
        table_name: "corridors".to_string(),
    };
    let result = exp.validate(&data);
    assert!(matches!(result.status, QualityStatus::Pass));
}

#[test]
fn test_not_null_expectation_fails_on_null() {
    let data = vec![json!({"asset_id": "USDC"}), json!({"asset_id": null})];
    let exp = NotNullExpectation {
        field: "asset_id".to_string(),
        table_name: "corridors".to_string(),
    };
    let result = exp.validate(&data);
    assert!(matches!(result.status, QualityStatus::Fail(_)));
}
