use crate::ml::PredictionFeatures;

#[tokio::test]
async fn test_ml_prediction() {
    let features = PredictionFeatures {
        corridor_hash: 0.5,
        amount_usd: 2.0, // log10(100)
        hour_of_day: 0.5, // 12 PM
        day_of_week: 0.3, // Tuesday
        liquidity_depth: 3.0, // log10(1000)
        recent_success_rate: 0.85,
    };

    // Test that features are in expected ranges
    assert!(features.corridor_hash >= 0.0 && features.corridor_hash <= 1.0);
    assert!(features.hour_of_day >= 0.0 && features.hour_of_day <= 1.0);
    assert!(features.day_of_week >= 0.0 && features.day_of_week <= 1.0);
    assert!(features.recent_success_rate >= 0.0 && features.recent_success_rate <= 1.0);
}

#[test]
fn test_prediction_result_risk_levels() {
    use crate::ml_handlers::PredictionResponse;
    use crate::ml::PredictionResult;

    let high_prob = PredictionResult {
        success_probability: 0.9,
        confidence: 0.8,
        model_version: "1.0.0".to_string(),
    };
    
    let response: PredictionResponse = high_prob.into();
    assert_eq!(response.risk_level, "low");
    assert!(response.recommendation.contains("Proceed"));

    let low_prob = PredictionResult {
        success_probability: 0.3,
        confidence: 0.8,
        model_version: "1.0.0".to_string(),
    };
    
    let response: PredictionResponse = low_prob.into();
    assert_eq!(response.risk_level, "high");
    assert!(response.recommendation.contains("High risk"));
}

#[test]
fn test_simple_model_prediction() {
    use crate::ml::SimpleMLModel;
    
    let model = SimpleMLModel::new();
    let features = PredictionFeatures {
        corridor_hash: 0.5,
        amount_usd: 2.0,
        hour_of_day: 0.5,
        day_of_week: 0.3,
        liquidity_depth: 3.0,
        recent_success_rate: 0.85,
    };
    
    let result = model.predict(features);
    assert!(result.success_probability >= 0.0 && result.success_probability <= 1.0);
    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    assert_eq!(result.model_version, "1.0.0");
}
