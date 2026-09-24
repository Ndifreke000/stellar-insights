use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_full_payment_flow() {
    // Initialize HTTP client
    let client = Client::new();
    let backend_url =
        std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    // Step 1: Fund testnet account
    let test_keypair = stellar_base::crypto::KeyPair::random();
    let public_key = test_keypair.public_key().account_id();

    let friendbot_url = format!("https://friendbot.stellar.org/?addr={}", public_key);
    let friendbot_response = client
        .get(&friendbot_url)
        .send()
        .await
        .expect("Failed to call Friendbot");

    assert!(
        friendbot_response.status().is_success(),
        "Friendbot funding failed: {}",
        friendbot_response.status()
    );

    // Step 2: SEP-10 Authentication
    // Call backend SEP-10 challenge endpoint
    let challenge_url = format!("{}/auth?account={}", backend_url, public_key);
    let challenge_response = client
        .get(&challenge_url)
        .send()
        .await
        .expect("Failed to get SEP-10 challenge");

    assert!(
        challenge_response.status().is_success(),
        "SEP-10 challenge endpoint returned: {}",
        challenge_response.status()
    );

    let challenge_data: Value = challenge_response
        .json()
        .await
        .expect("Failed to parse challenge response");

    let challenge_xdr = challenge_data
        .get("transaction")
        .and_then(|v| v.as_str())
        .expect("Challenge response missing 'transaction' field");

    // Parse and sign the challenge transaction
    let envelope = stellar_base::TransactionEnvelope::from_xdr(challenge_xdr)
        .expect("Failed to parse challenge XDR");

    let tx = match envelope {
        stellar_base::TransactionEnvelope::Tx(tx) => tx,
        _ => panic!("Expected Tx envelope"),
    };

    let signed_envelope = tx
        .into_transaction()
        .into_envelope(&test_keypair)
        .expect("Failed to sign challenge");

    let signed_xdr = signed_envelope
        .to_xdr()
        .expect("Failed to serialize signed envelope");

    // Submit signed challenge
    let submit_url = format!("{}/auth", backend_url);
    let submit_response = client
        .post(&submit_url)
        .json(&json!({
            "transaction": signed_xdr
        }))
        .send()
        .await
        .expect("Failed to submit signed challenge");

    assert!(
        submit_response.status().is_success(),
        "Challenge submission failed: {}",
        submit_response.status()
    );

    let submit_data: Value = submit_response
        .json()
        .await
        .expect("Failed to parse auth submission response");

    let jwt = submit_data
        .get("token")
        .and_then(|v| v.as_str())
        .expect("Auth response missing 'token' field");

    // Step 3: SEP-24 Deposit
    let deposit_url = format!("{}/api/sep24/deposit/interactive", backend_url);
    let deposit_response = client
        .post(&deposit_url)
        .header("Authorization", format!("Bearer {}", jwt))
        .json(&json!({
            "transfer_server": "https://anchor.example.com",
            "asset_code": "USDC",
            "account": public_key,
            "amount": "100.00"
        }))
        .send()
        .await
        .expect("Failed to initiate SEP-24 deposit");

    assert!(
        deposit_response.status().is_success(),
        "SEP-24 deposit endpoint returned: {}",
        deposit_response.status()
    );

    let deposit_data: Value = deposit_response
        .json()
        .await
        .expect("Failed to parse deposit response");

    let transaction_id = deposit_data
        .get("id")
        .or_else(|| deposit_data.get("transaction_id"))
        .and_then(|v| v.as_str())
        .expect("Deposit response missing transaction ID");

    // Step 4: Wait for dashboard appearance (poll every 2s for up to 30s)
    let dashboard_url = format!("{}/api/analytics/transactions", backend_url);
    let mut attempt = 0;
    let max_attempts = 15; // 30 seconds / 2 seconds per attempt

    loop {
        sleep(Duration::from_secs(2)).await;
        attempt += 1;

        let dashboard_response = client
            .get(&dashboard_url)
            .header("Authorization", format!("Bearer {}", jwt))
            .send()
            .await;

        if let Ok(response) = dashboard_response {
            if let Ok(data) = response.json::<Value>().await {
                // Check if transaction appears in the response
                if let Some(transactions) = data.get("transactions").and_then(|v| v.as_array()) {
                    if transactions.iter().any(|tx| {
                        tx.get("id")
                            .and_then(|v| v.as_str())
                            .map(|id| id == transaction_id)
                            .unwrap_or(false)
                    }) {
                        // Transaction found on dashboard
                        return;
                    }
                }
            }
        }

        if attempt >= max_attempts {
            panic!(
                "Transaction {} did not appear on dashboard within 30 seconds",
                transaction_id
            );
        }
    }
}
