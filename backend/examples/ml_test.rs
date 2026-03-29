use anyhow::Result;
use reqwest::Client;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();

    // Get API base URL from environment or use default for development
    let api_base = env::var("API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    // Test prediction endpoint
    let response = client
        .get(&format!(
            "{}/api/ml/predict?corridor=USDC-USD&amount_usd=100.0",
            api_base
        ))
        .send()
        .await?;

    if response.status().is_success() {
        let prediction: serde_json::Value = response.json().await?;
        println!(
            "Prediction result: {}",
            serde_json::to_string_pretty(&prediction)?
        );
    } else {
        println!("Error: {}", response.status());
    }

    // Test model status
    let status_response = client
        .get(&format!("{}/api/ml/status", api_base))
        .send()
        .await?;

    if status_response.status().is_success() {
        let status: serde_json::Value = status_response.json().await?;
        println!("Model status: {}", serde_json::to_string_pretty(&status)?);
    }

    Ok(())
}
