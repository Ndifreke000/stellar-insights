use reqwest::Client;
use std::env;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use url::Url;

fn get_config() -> (String, String) {
    let url = env::var("TESTNET_API_URL").expect("TESTNET_API_URL must be set");
    let key = env::var("TESTNET_API_KEY").expect("TESTNET_API_KEY must be set");
    
    // Strip trailing slash if present
    let url = url.trim_end_matches('/').to_string();
    (url, key)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (url, _) = get_config();
    let client = Client::new();
    
    let res = client
        .get(format!("{}/health", url))
        .send()
        .await
        .expect("Failed to send request");
        
    assert!(res.status().is_success(), "Health endpoint is not OK");
}

#[tokio::test]
async fn test_auth_endpoint() {
    let (url, key) = get_config();
    let client = Client::new();
    
    let res = client
        .post(format!("{}/api/auth/login", url))
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .expect("Failed to send request");
        
    // As long as it's not a 5xx or 404 error, the endpoint is reachable
    let status = res.status().as_u16();
    assert!(
        status < 500 && status != 404,
        "Auth endpoint returned unexpected status: {}",
        status
    );
}

#[tokio::test]
async fn test_transactions_endpoint() {
    let (url, key) = get_config();
    let client = Client::new();
    
    let res = client
        .get(format!("{}/api/transactions/", url))
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .expect("Failed to send request");
        
    let status = res.status().as_u16();
    assert!(
        status < 500 && status != 404,
        "Transactions endpoint returned unexpected status: {}",
        status
    );
}

#[tokio::test]
async fn test_analytics_endpoint() {
    let (url, key) = get_config();
    let client = Client::new();
    
    let res = client
        .get(format!("{}/api/analytics/verification-summary", url))
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .expect("Failed to send request");
        
    let status = res.status().as_u16();
    assert!(
        status < 500 && status != 404,
        "Analytics endpoint returned unexpected status: {}",
        status
    );
}

#[tokio::test]
async fn test_websocket_endpoint() {
    let (http_url, _) = get_config();
    let ws_url = if http_url.starts_with("https://") {
        http_url.replace("https://", "wss://")
    } else {
        http_url.replace("http://", "ws://")
    };
    
    let url = Url::parse(&format!("{}/ws", ws_url)).expect("Invalid WS URL");
    
    let ws_connect = tokio::time::timeout(
        Duration::from_secs(10),
        connect_async(url.clone())
    ).await;
    
    match ws_connect {
        Ok(Ok((_ws_stream, _response))) => {
            // Connected successfully
            assert!(true);
        }
        Ok(Err(e)) => {
            // A non-200 HTTP response during WS handshake (like 401 or 403) is fine for a smoke test
            // as it means the server is reachable and properly rejecting unauthenticated WS connections.
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("401") || err_msg.contains("403") || err_msg.contains("400") || err_msg.contains("Handshake"),
                "WebSocket connection failed with unexpected error: {}",
                e
            );
        }
        Err(_) => {
            panic!("WebSocket connection timed out");
        }
    }
}
