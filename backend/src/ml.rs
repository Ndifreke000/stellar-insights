use chrono::{DateTime, Utc, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use crate::database::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionFeatures {
    pub corridor_hash: f32,
    pub amount_usd: f32,
    pub hour_of_day: f32,
    pub day_of_week: f32,
    pub liquidity_depth: f32,
    pub recent_success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub success_probability: f32,
    pub confidence: f32,
    pub model_version: String,
}

#[derive(Debug, Clone)]
pub struct SimpleMLModel {
    weights: Vec<f32>,
    bias: f32,
    version: String,
}

impl SimpleMLModel {
    pub fn new() -> Self {
        // Simple linear model weights (trained offline)
        Self {
            weights: vec![0.1, 0.3, 0.05, 0.02, 0.4, 0.6], // 6 features
            bias: 0.2,
            version: "1.0.0".to_string(),
        }
    }

    pub fn predict(&self, features: PredictionFeatures) -> PredictionResult {
        let input = vec![
            features.corridor_hash,
            features.amount_usd,
            features.hour_of_day,
            features.day_of_week,
            features.liquidity_depth,
            features.recent_success_rate,
        ];

        let mut score = self.bias;
        for (i, &weight) in self.weights.iter().enumerate() {
            score += weight * input[i];
        }

        // Sigmoid activation
        let prob = 1.0 / (1.0 + (-score).exp());
        
        PredictionResult {
            success_probability: prob,
            confidence: if prob > 0.7 || prob < 0.3 { 0.9 } else { 0.7 },
            model_version: self.version.clone(),
        }
    }

    pub fn train(&mut self, _training_data: &[(Vec<f32>, f32)]) {
        // Simple gradient descent (placeholder)
        // In production, this would implement actual training
        println!("Training model with {} samples", _training_data.len());
        
        // Update version after training
        self.version = format!("1.0.{}", chrono::Utc::now().timestamp() % 1000);
    }
}

pub struct MLService {
    model: SimpleMLModel,
    #[allow(dead_code)]
    db: Database,
}

impl MLService {
    pub fn new(db: Database) -> anyhow::Result<Self> {
        let model = SimpleMLModel::new();
        Ok(Self { model, db })
    }

    pub async fn train_model(&mut self) -> anyhow::Result<()> {
        let training_data = self.prepare_training_data().await?;
        self.model.train(&training_data);
        Ok(())
    }

    async fn prepare_training_data(&self) -> anyhow::Result<Vec<(Vec<f32>, f32)>> {
        // Mock training data for now
        let mut training_data = Vec::new();
        
        // Generate some sample data
        for i in 0..1000 {
            let features = vec![
                (i % 100) as f32 / 100.0,  // corridor_hash
                (i % 50) as f32 / 10.0,    // amount (log scale)
                (i % 24) as f32 / 24.0,    // hour
                (i % 7) as f32 / 7.0,      // day
                5.0 + (i % 10) as f32,     // liquidity
                0.7 + (i % 30) as f32 / 100.0, // success rate
            ];
            
            // Simple target: higher success rate = higher probability
            let target = if features[5] > 0.8 { 1.0 } else { 0.0 };
            training_data.push((features, target));
        }

        Ok(training_data)
    }

    fn hash_corridor(&self, asset_code: &Option<String>, asset_issuer: &Option<String>) -> f32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        asset_code.hash(&mut hasher);
        asset_issuer.hash(&mut hasher);
        (hasher.finish() % 1000) as f32 / 1000.0
    }

    pub async fn predict_payment_success(
        &self,
        corridor: &str,
        amount_usd: f64,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<PredictionResult> {
        let parts: Vec<&str> = corridor.split('-').collect();
        let corridor_hash = self.hash_corridor(
            &Some(parts.get(0).unwrap_or(&"").to_string()),
            &Some(parts.get(1).unwrap_or(&"").to_string()),
        );

        let liquidity = self.get_corridor_liquidity(corridor).await.unwrap_or(1000.0);
        let recent_success = self.get_recent_success_rate(corridor).await.unwrap_or(0.8);

        let features = PredictionFeatures {
            corridor_hash,
            amount_usd: amount_usd.log10().max(0.0) as f32,
            hour_of_day: timestamp.hour() as f32 / 24.0,
            day_of_week: timestamp.weekday().num_days_from_monday() as f32 / 7.0,
            liquidity_depth: liquidity.log10() as f32,
            recent_success_rate: recent_success,
        };

        Ok(self.model.predict(features))
    }

    async fn get_corridor_liquidity(&self, corridor: &str) -> Option<f64> {
        // Mock data for now - in production this would query the database
        Some(1000.0 + (corridor.len() as f64 * 100.0))
    }

    async fn get_recent_success_rate(&self, corridor: &str) -> Option<f32> {
        // Mock data for now - in production this would query the database  
        Some(0.8 + (corridor.len() as f32 * 0.01) % 0.2)
    }

    pub async fn retrain_weekly(&mut self) -> anyhow::Result<()> {
        println!("Starting weekly model retraining...");
        self.train_model().await?;
        
        println!("Model retrained successfully. Version: {}", self.model.version);
        Ok(())
    }
}
