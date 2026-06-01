# ML Payment Success Prediction

This module implements a machine learning model to predict payment success probability based on historical Stellar network data.

## Features

- **Neural Network Model**: 3-layer feedforward network using Candle ML framework
- **Feature Engineering**: Corridor hash, amount, time patterns, liquidity depth, recent success rates
- **Real-time Predictions**: REST API endpoint for instant predictions
- **Automatic Retraining**: Weekly model updates with latest data
- **Model Versioning**: Track model versions and performance metrics

## API Endpoints

### Predict Payment Success
```
GET /api/ml/predict?corridor=USDC-USD&amount_usd=100.0&timestamp=2024-01-01T12:00:00Z
```

Response:
```json
{
  "success_probability": 0.87,
  "confidence": 0.9,
  "model_version": "1.0.0",
  "risk_level": "low",
  "recommendation": "Proceed with payment"
}
```

### Model Status
```
GET /api/ml/status
```

Response:
```json
{
  "version": "1.0.0",
  "last_trained": "2024-01-01",
  "accuracy": 0.87,
  "total_predictions": 1000
}
```

### Manual Retrain
```
POST /api/ml/retrain
```

## Model Architecture

- **Input Features (6)**:
  - Corridor hash (normalized)
  - Amount USD (log10 scaled)
  - Hour of day (0-1)
  - Day of week (0-1)
  - Liquidity depth (log10 scaled)
  - Recent success rate (0-1)

- **Hidden Layers**: 32 → 16 neurons with ReLU activation
- **Output**: Single probability (0-1) with sigmoid activation

## Training Data

- **Source**: 90 days of payment records from Stellar network
- **Features**: Extracted from payment metadata and corridor metrics
- **Target**: Binary success/failure labels
- **Size**: Up to 10,000 recent payments

## Performance Targets

- **Accuracy**: >85% on test set
- **Training**: Weekly automatic retraining
- **Inference**: <100ms response time
- **Availability**: 99.9% uptime

## Usage Example

```rust
use stellar_insights_backend::ml::MLService;

let ml_service = MLService::new(database).await?;
let prediction = ml_service
    .predict_payment_success("USDC-USD", 100.0, Utc::now())
    .await?;

println!("Success probability: {:.2}%", prediction.success_probability * 100.0);
```

## Testing

Run ML tests:
```bash
cargo test ml_tests
```

Test with example client:
```bash
cargo run --example ml_test
```

## Configuration

Environment variables:
- `ML_MODEL_PATH`: Path to save/load model weights
- `ML_RETRAIN_INTERVAL`: Retraining interval in seconds (default: 604800 = 7 days)
- `ML_BATCH_SIZE`: Training batch size (default: 32)
- `ML_LEARNING_RATE`: Learning rate (default: 0.001)
