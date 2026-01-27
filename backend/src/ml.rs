use candle_core::{Device, Result, Tensor};
use candle_nn::{linear, Linear, Module, Optimizer, VarBuilder, VarMap};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
pub struct MLModel {
    linear1: Linear,
    linear2: Linear,
    linear3: Linear,
    device: Device,
    version: String,
}

impl MLModel {
    pub fn new(device: Device) -> Result<Self> {
        let mut varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, candle_core::DType::F32, &device);
        
        let linear1 = linear(6, 32, vs.pp("l1"))?;
        let linear2 = linear(32, 16, vs.pp("l2"))?;
        let linear3 = linear(16, 1, vs.pp("l3"))?;
        
        Ok(Self {
            linear1,
            linear2,
            linear3,
            device,
            version: "1.0.0".to_string(),
        })
    }

    pub fn forward(&self, features: &Tensor) -> Result<Tensor> {
        let x = self.linear1.forward(features)?;
        let x = x.relu()?;
        let x = self.linear2.forward(&x)?;
        let x = x.relu()?;
        let x = self.linear3.forward(&x)?;
        x.sigmoid()
    }

    pub fn predict(&self, features: PredictionFeatures) -> Result<PredictionResult> {
        let input = Tensor::new(
            &[
                features.corridor_hash,
                features.amount_usd,
                features.hour_of_day,
                features.day_of_week,
                features.liquidity_depth,
                features.recent_success_rate,
            ],
            &self.device,
        )?
        .unsqueeze(0)?;

        let output = self.forward(&input)?;
        let prob = output.to_vec1::<f32>()?[0];
        
        Ok(PredictionResult {
            success_probability: prob,
            confidence: if prob > 0.7 || prob < 0.3 { 0.9 } else { 0.7 },
            model_version: self.version.clone(),
        })
    }
}

pub struct MLService {
    model: MLModel,
    db: Database,
}

impl MLService {
    pub fn new(db: Database) -> Result<Self> {
        let device = Device::Cpu;
        let model = MLModel::new(device)?;
        
        Ok(Self { model, db })
    }

    pub async fn train_model(&mut self) -> anyhow::Result<()> {
        let training_data = self.prepare_training_data().await?;
        
        // Simple training loop
        let mut varmap = VarMap::new();
        let device = Device::Cpu;
        let vs = VarBuilder::from_varmap(&varmap, candle_core::DType::F32, &device);
        
        let model = MLModel::new(device)?;
        let mut optimizer = candle_nn::AdamW::new(varmap.all_vars(), Default::default())?;
        
        for epoch in 0..100 {
            let mut total_loss = 0.0;
            
            for (features, target) in &training_data {
                let pred = model.forward(features)?;
                let loss = (pred - target)?.sqr()?.mean_all()?;
                
                optimizer.backward_step(&loss)?;
                total_loss += loss.to_scalar::<f32>()?;
            }
            
            if epoch % 20 == 0 {
                println!("Epoch {}: Loss = {:.4}", epoch, total_loss / training_data.len() as f32);
            }
        }
        
        self.model = model;
        Ok(())
    }

    async fn prepare_training_data(&self) -> anyhow::Result<Vec<(Tensor, Tensor)>> {
        let payments = sqlx::query!(
            r#"
            SELECT 
                p.amount::float as amount,
                p.created_at,
                p.asset_code,
                p.asset_issuer,
                CASE WHEN p.id IS NOT NULL THEN 1.0 ELSE 0.0 END as success
            FROM payment_records p
            WHERE p.created_at >= NOW() - INTERVAL '90 days'
            ORDER BY p.created_at DESC
            LIMIT 10000
            "#
        )
        .fetch_all(&self.db.pool)
        .await?;

        let mut training_data = Vec::new();
        let device = Device::Cpu;

        for payment in payments {
            let corridor_hash = self.hash_corridor(&payment.asset_code, &payment.asset_issuer);
            let hour = payment.created_at.hour() as f32 / 24.0;
            let day = payment.created_at.weekday().num_days_from_monday() as f32 / 7.0;
            
            let features = Tensor::new(
                &[
                    corridor_hash,
                    payment.amount.log10().max(0.0),
                    hour,
                    day,
                    0.5, // placeholder liquidity
                    0.8, // placeholder recent success rate
                ],
                &device,
            )?;
            
            let target = Tensor::new(&[payment.success], &device)?;
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

        self.model.predict(features).map_err(|e| anyhow::anyhow!("Prediction error: {}", e))
    }

    async fn get_corridor_liquidity(&self, corridor: &str) -> Option<f64> {
        sqlx::query_scalar!(
            "SELECT liquidity_depth_usd FROM corridor_records WHERE id = $1",
            corridor
        )
        .fetch_optional(&self.db.pool)
        .await
        .ok()
        .flatten()
    }

    async fn get_recent_success_rate(&self, corridor: &str) -> Option<f32> {
        sqlx::query_scalar!(
            r#"
            SELECT 
                COALESCE(
                    successful_payments::float / NULLIF(total_attempts, 0),
                    0.0
                ) as success_rate
            FROM corridor_records 
            WHERE id = $1
            "#,
            corridor
        )
        .fetch_optional(&self.db.pool)
        .await
        .ok()
        .flatten()
        .map(|r| r as f32)
    }

    pub async fn retrain_weekly(&mut self) -> anyhow::Result<()> {
        println!("Starting weekly model retraining...");
        self.train_model().await?;
        
        // Save model version
        let new_version = format!("1.0.{}", chrono::Utc::now().timestamp());
        self.model.version = new_version;
        
        println!("Model retrained successfully. Version: {}", self.model.version);
        Ok(())
    }
}
