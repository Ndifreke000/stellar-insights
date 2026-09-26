use anyhow::Result;
use payraider_backend::rpc::{Asset, Payment, StellarRpcClient, Trade};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("backend=info")
        .init();

    println!("🚀 Stellar RPC Integration Demo\n");
    println!("================================\n");

    // Initialize client in mock mode for demo
    let mock_mode = std::env::var("MOCK_MODE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let client = StellarRpcClient::new_with_defaults(mock_mode);

    if mock_mode {
        println!("📊 Running in MOCK MODE (use MOCK_MODE=false for real data)\n");
    } else {
        println!("🌐 Connecting to LIVE Stellar Network\n");
    }

    // 1. Health Check
    println!("1️⃣  Checking RPC Health...");
    match client.check_health().await {
        Ok(health) => {
            println!("   ✅ Status: {}", health.status);
            println!("   📈 Latest Ledger: {}", health.latest_ledger);
            println!("   📉 Oldest Ledger: {}", health.oldest_ledger);
            println!(
                "   🔢 Retention Window: {} ledgers\n",
                health.ledger_retention_window
            );
        }
        Err(e) => {
            println!("   ❌ Error: {}\n", e);
        }
    }

    // 2. Fetch Latest Ledger
    println!("2️⃣  Fetching Latest Ledger...");
    match client.fetch_latest_ledger().await {
        Ok(ledger) => {
            println!("   ✅ Sequence: {}", ledger.sequence);
            println!("   🔗 Hash: {}", ledger.hash);
            println!("   💱 Transactions: {}", ledger.transaction_count);
            println!("   ⚙️  Operations: {}", ledger.operation_count);
            println!("   🕐 Closed At: {}\n", ledger.closed_at);
        }
        Err(e) => {
            println!("   ❌ Error: {}\n", e);
        }
    }

    // 3. Fetch Recent Payments
    println!("3️⃣  Fetching Recent Payments (limit: 5)...");
    match client.fetch_payments(5, None).await {
        Ok(payments) => {
            let payments: std::vec::Vec<Payment> = payments;
            println!("   ✅ Found {} payments:", payments.len());
            for (i, payment) in payments.iter().enumerate().take(3) {
                println!("   \n   Payment #{}:", i + 1);
                println!("     ID: {}", payment.id);
                println!(
                    "     Amount: {} {}",
                    payment.amount,
                    payment.asset_code.as_deref().unwrap_or("XLM")
                );
                println!(
                    "     From: {}...{}",
                    &payment.source_account[..10],
                    &payment.source_account[payment.source_account.len() - 5..]
                );
                println!(
                    "     To: {}...{}",
                    &payment.destination[..10],
                    &payment.destination[payment.destination.len() - 5..]
                );
            }
            println!();
        }
        Err(e) => {
            println!("   ❌ Error: {}\n", e);
        }
    }

    // 4. Fetch Recent Trades
    println!("4️⃣  Fetching Recent Trades (limit: 3)...");
    match client.fetch_trades(3, None).await {
        Ok(trades) => {
            let trades: std::vec::Vec<Trade> = trades;
            println!("   ✅ Found {} trades:", trades.len());
            for (i, trade) in trades.iter().enumerate() {
                println!("   \n   Trade #{}:", i + 1);
                println!("     ID: {}", trade.id);
                println!(
                    "     Base: {} {}",
                    trade.base_amount,
                    trade.base_asset_code.as_deref().unwrap_or("XLM")
                );
                println!(
                    "     Counter: {} {}",
                    trade.counter_amount,
                    trade.counter_asset_code.as_deref().unwrap_or("XLM")
                );
                println!("     Price: {}/{}", trade.price.n, trade.price.d);
                println!("     Time: {}", trade.ledger_close_time);
            }
            println!();
        }
        Err(e) => {
            println!("   ❌ Error: {}\n", e);
        }
    }

    // 5. Fetch Order Book
    println!("5️⃣  Fetching Order Book (XLM/USDC)...");
    let selling_asset = Asset {
        asset_type: "native".to_string(),
        asset_code: None,
        asset_issuer: None,
    };

    let buying_asset = Asset {
        asset_type: "credit_alphanum4".to_string(),
        asset_code: Some("USDC".to_string()),
        asset_issuer: Some("GBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string()),
    };

    match client
        .fetch_order_book(&selling_asset, &buying_asset, 10)
        .await
    {
        Ok(order_book) => {
            println!("   ✅ Order Book Retrieved:");
            println!("   \n   📗 Top Bids:");
            for (i, bid) in order_book.bids.iter().take(3).enumerate() {
                println!(
                    "     {}. Price: {} | Amount: {}",
                    i + 1,
                    bid.price,
                    bid.amount
                );
            }
            println!("   \n   📕 Top Asks:");
            for (i, ask) in order_book.asks.iter().take(3).enumerate() {
                println!(
                    "     {}. Price: {} | Amount: {}",
                    i + 1,
                    ask.price,
                    ask.amount
                );
            }
            println!();
        }
        Err(e) => {
            println!("   ❌ Error: {}\n", e);
        }
    }

    println!("================================");
    println!("✨ Demo Complete!\n");

    if mock_mode {
        println!("💡 Tip: Set MOCK_MODE=false to fetch real data from Stellar network");
    } else {
        println!("✅ Successfully connected to live Stellar network!");
    }

    Ok(())
}
