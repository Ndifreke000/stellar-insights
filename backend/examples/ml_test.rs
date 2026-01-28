use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();
    
    // Test prediction endpoint
    let response = client
        .get("http://localhost:8080/api/ml/predict")
        .query(&[
            ("corridor", "USDC-USD"),
            ("amount_usd", "100.0"),
        ])
        .send()
        .await?;

    if response.status().is_success() {
        let prediction: serde_json::Value = response.json().await?;
        println!("Prediction result: {}", serde_json::to_string_pretty(&prediction)?);
    } else {
        println!("Error: {}", response.status());
    }

    // Test model status
    let status_response = client
        .get("http://localhost:8080/api/ml/status")
        .send()
        .await?;

    if status_response.status().is_success() {
        let status: serde_json::Value = status_response.json().await?;
        println!("Model status: {}", serde_json::to_string_pretty(&status)?);
    }

    Ok(())
}
