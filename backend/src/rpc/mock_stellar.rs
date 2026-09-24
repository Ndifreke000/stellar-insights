//! Deterministic Stellar Horizon/RPC fixtures for tests and mock-mode clients.

use super::stellar::{
    Asset, AssetAccounts, AssetBalanceChange, AssetBalances, AssetFlags, FeeBumpTransactionInfo,
    GetLedgersResult, HealthResponse, HorizonAsset, HorizonEffect, HorizonLiquidityPool,
    HorizonOperation, HorizonPoolReserve, HorizonTransaction, InnerTransaction, LedgerInfo,
    OrderBook, OrderBookEntry, Payment, Price, RpcLedger, Trade,
};

pub const MOCK_OLDEST_LEDGER: u64 = 51_565_760;
pub const MOCK_LATEST_LEDGER: u64 = 51_565_820;
pub fn mock_health_response() -> HealthResponse {
    HealthResponse {
        status: "healthy".to_string(),
        latest_ledger: MOCK_LATEST_LEDGER,
        oldest_ledger: MOCK_OLDEST_LEDGER,
        ledger_retention_window: 60,
    }
}

pub fn mock_ledger_info() -> LedgerInfo {
    LedgerInfo {
        sequence: 51_583_040,
        hash: "abc123def456".to_string(),
        previous_hash: "xyz789uvw012".to_string(),
        transaction_count: 245,
        operation_count: 1203,
        closed_at: "2026-01-22T10:30:00Z".to_string(),
        total_coins: "105443902087.3472865".to_string(),
        fee_pool: "3145678.9012345".to_string(),
        base_fee: 100,
        base_reserve: "0.5".to_string(),
        protocol_version: 21,
    }
}

// I'm mocking getLedgers response for testing
pub fn mock_get_ledgers(start: u64, limit: u32) -> GetLedgersResult {
    if start > MOCK_LATEST_LEDGER {
        return GetLedgersResult {
            ledgers: Vec::new(),
            latest_ledger: MOCK_LATEST_LEDGER,
            oldest_ledger: MOCK_OLDEST_LEDGER,
            cursor: Some(MOCK_LATEST_LEDGER.to_string()),
        };
    }

    let end = (start.saturating_add(u64::from(limit)).saturating_sub(1)).min(MOCK_LATEST_LEDGER);
    let ledgers = (start..=end)
        .enumerate()
        .map(|(i, seq)| RpcLedger {
            hash: format!("hash_{seq}"),
            sequence: seq,
            ledger_close_time: format!("{}", 1_734_032_457 + i as u64 * 5),
            header_xdr: Some("mock_header".to_string()),
            metadata_xdr: Some("mock_metadata".to_string()),
        })
        .collect();

    GetLedgersResult {
        ledgers,
        latest_ledger: MOCK_LATEST_LEDGER,
        oldest_ledger: MOCK_OLDEST_LEDGER,
        cursor: Some(end.to_string()),
    }
}

pub fn mock_payments(limit: u32) -> Vec<Payment> {
    (0..limit)
        .map(|i| {
            let is_path_payment = i % 5 == 0;
            let is_native_source = i % 3 == 0;
            let is_native_dest = i % 4 == 0;
            // Use the new Horizon format for even-indexed entries so
            // tests exercise both the legacy and new code paths.
            let use_new_format = i % 2 == 0;

            let dest_account = format!("GDYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY{i:03}");
            let src_account = format!("GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX{i:03}");
            let asset_type_str = if is_native_dest {
                "native".to_string()
            } else if i % 2 == 0 {
                "credit_alphanum4".to_string()
            } else {
                "credit_alphanum12".to_string()
            };
            let asset_code_val = if is_native_dest {
                None
            } else if i % 2 == 0 {
                Some(["USDC", "EURT", "BRL", "NGNT"][i as usize % 4].to_string())
            } else {
                Some("LONGASSETCODE".to_string())
            };
            let asset_issuer_val = if is_native_dest {
                None
            } else {
                Some(format!(
                    "GISSUER{:02}XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                    i % 10
                ))
            };
            let amount_str = format!("{}.0000000", 100 + i * 10);

            Payment {
                id: format!("payment_{i}"),
                paging_token: format!("paging_{i}"),
                transaction_hash: format!("txhash_{i}"),
                source_account: src_account.clone(),
                // When the new format is used the top-level destination may
                // be empty, just like the real Horizon response.
                destination: if use_new_format {
                    String::new()
                } else {
                    dest_account.clone()
                },
                asset_type: asset_type_str.clone(),
                asset_code: if use_new_format {
                    None
                } else {
                    asset_code_val.clone()
                },
                asset_issuer: if use_new_format {
                    None
                } else {
                    asset_issuer_val.clone()
                },
                amount: if use_new_format {
                    String::new()
                } else {
                    amount_str.clone()
                },
                created_at: format!("2026-01-22T10:{:02}:00Z", i % 60),
                operation_type: if is_path_payment {
                    Some(if i % 2 == 0 {
                        "path_payment_strict_send".to_string()
                    } else {
                        "path_payment_strict_receive".to_string()
                    })
                } else {
                    Some("payment".to_string())
                },
                // Source asset for path payments
                source_asset_type: if is_path_payment {
                    Some(if is_native_source {
                        "native".to_string()
                    } else {
                        "credit_alphanum4".to_string()
                    })
                } else {
                    None
                },
                source_asset_code: if is_path_payment && !is_native_source {
                    Some(["USD", "EUR", "GBP", "JPY"][i as usize % 4].to_string())
                } else {
                    None
                },
                source_asset_issuer: if is_path_payment && !is_native_source {
                    Some(format!(
                        "GSRCISSUER{:02}XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                        i % 10
                    ))
                } else {
                    None
                },
                source_amount: if is_path_payment {
                    Some(format!("{}.0000000", 90 + i * 10))
                } else {
                    None
                },
                from: Some(src_account),
                to: Some(dest_account.clone()),
                // Populate the new Soroban-compatible field for even entries
                asset_balance_changes: if use_new_format {
                    Some(vec![AssetBalanceChange {
                        asset_type: asset_type_str,
                        asset_code: asset_code_val,
                        asset_issuer: asset_issuer_val,
                        change_type: "transfer".to_string(),
                        from: Some(format!(
                            "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX{i:03}"
                        )),
                        to: Some(dest_account),
                        amount: amount_str,
                    }])
                } else {
                    None
                },
            }
        })
        .collect()
}

pub fn mock_trades(limit: u32) -> Vec<Trade> {
    (0..limit)
        .map(|i| Trade {
            id: format!("trade_{i}"),
            ledger_close_time: format!("2026-01-22T10:{:02}:00Z", i % 60),
            base_account: format!("GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX{i:03}"),
            base_amount: format!("{}.0000000", 1000 + i * 100),
            base_asset_type: "native".to_string(),
            base_asset_code: None,
            base_asset_issuer: None,
            counter_account: format!("GDYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY{i:03}"),
            counter_amount: format!("{}.0000000", 500 + i * 50),
            counter_asset_type: "credit_alphanum4".to_string(),
            counter_asset_code: Some("USDC".to_string()),
            counter_asset_issuer: Some(
                "GBXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            ),
            price: Price {
                n: 2 + i64::from(i),
                d: 1,
            },
            trade_type: "orderbook".to_string(),
        })
        .collect()
}

pub fn mock_order_book(selling_asset: &Asset, buying_asset: &Asset) -> OrderBook {
    let bids = vec![
        OrderBookEntry {
            price: "0.9950".to_string(),
            amount: "1000.0000000".to_string(),
        },
        OrderBookEntry {
            price: "0.9900".to_string(),
            amount: "2500.0000000".to_string(),
        },
        OrderBookEntry {
            price: "0.9850".to_string(),
            amount: "5000.0000000".to_string(),
        },
    ];

    let asks = vec![
        OrderBookEntry {
            price: "1.0050".to_string(),
            amount: "1200.0000000".to_string(),
        },
        OrderBookEntry {
            price: "1.0100".to_string(),
            amount: "3000.0000000".to_string(),
        },
        OrderBookEntry {
            price: "1.0150".to_string(),
            amount: "4500.0000000".to_string(),
        },
    ];

    OrderBook {
        bids,
        asks,
        base: selling_asset.clone(),
        counter: buying_asset.clone(),
    }
}

pub fn mock_transactions(limit: u32, ledger_sequence: u64) -> Vec<HorizonTransaction> {
    (0..limit)
        .map(|i| {
            let is_fee_bump = i % 2 == 0;
            HorizonTransaction {
                id: format!("tx_{i}"),
                hash: format!("txhash_{i}"),
                ledger: ledger_sequence,
                created_at: "2026-01-22T10:30:00Z".to_string(),
                source_account: "GXX".to_string(),
                fee_account: Some("GXX".to_string()),
                fee_charged: Some("100".to_string()),
                max_fee: Some("1000".to_string()),
                operation_count: 1,
                successful: true,
                paging_token: format!("pt_{i}"),
                fee_bump_transaction: if is_fee_bump {
                    Some(FeeBumpTransactionInfo {
                        hash: format!("fb_hash_{i}"),
                        signatures: vec!["sig1".to_string()],
                    })
                } else {
                    None
                },
                inner_transaction: if is_fee_bump {
                    Some(InnerTransaction {
                        hash: format!("inner_hash_{i}"),
                        max_fee: Some("500".to_string()),
                        signatures: vec!["sig1".to_string()],
                    })
                } else {
                    None
                },
            }
        })
        .collect()
}

pub fn mock_operations_for_ledger(sequence: u64) -> Vec<HorizonOperation> {
    let source_a = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
    let source_b = "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();
    let dest_a = "GDESTAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
    let dest_b = "GDESTBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();

    vec![
        HorizonOperation {
            id: format!("op_{sequence}_0"),
            paging_token: format!("pt_{sequence}_0"),
            transaction_hash: format!("txhash_{sequence}_0"),
            source_account: source_a.clone(),
            operation_type: "account_merge".to_string(),
            created_at: "2026-01-22T10:30:00Z".to_string(),
            account: Some(source_a),
            into: Some(dest_a),
            amount: None,
        },
        HorizonOperation {
            id: format!("op_{sequence}_1"),
            paging_token: format!("pt_{sequence}_1"),
            transaction_hash: format!("txhash_{sequence}_1"),
            source_account: "GCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC".to_string(),
            operation_type: "payment".to_string(),
            created_at: "2026-01-22T10:31:00Z".to_string(),
            account: None,
            into: None,
            amount: Some("25.0000000".to_string()),
        },
        HorizonOperation {
            id: format!("op_{sequence}_2"),
            paging_token: format!("pt_{sequence}_2"),
            transaction_hash: format!("txhash_{sequence}_2"),
            source_account: source_b.clone(),
            operation_type: "account_merge".to_string(),
            created_at: "2026-01-22T10:32:00Z".to_string(),
            account: Some(source_b),
            into: Some(dest_b),
            amount: None,
        },
    ]
}

pub fn mock_effects_for_operation(operation_id: &str) -> Vec<HorizonEffect> {
    if operation_id.ends_with("_0") {
        return vec![HorizonEffect {
            id: format!("effect_{operation_id}_0"),
            effect_type: "account_credited".to_string(),
            account: Some("GDESTAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()),
            amount: Some("125.5000000".to_string()),
            asset_type: Some("native".to_string()),
        }];
    }

    if operation_id.ends_with("_2") {
        return vec![
            HorizonEffect {
                id: format!("effect_{operation_id}_0"),
                effect_type: "account_credited".to_string(),
                account: Some(
                    "GDESTBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string(),
                ),
                amount: Some("10.0000000".to_string()),
                asset_type: Some("native".to_string()),
            },
            HorizonEffect {
                id: format!("effect_{operation_id}_1"),
                effect_type: "account_credited".to_string(),
                account: Some(
                    "GDESTBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string(),
                ),
                amount: Some("0.5000000".to_string()),
                asset_type: Some("native".to_string()),
            },
        ];
    }

    Vec::new()
}
pub fn mock_liquidity_pools(limit: u32) -> Vec<HorizonLiquidityPool> {
    let pool_configs = [
        (
            "USDC",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            "XLM",
            "",
            "500000.0",
            "1200000.0",
            "850000.0",
        ),
        (
            "USDC",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            "EURC",
            "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y36DAVIZA67CE7BKBHP4V2OA",
            "320000.0",
            "295000.0",
            "610000.0",
        ),
        (
            "XLM",
            "",
            "BTC",
            "GDPJALI4AZKUU2W426U5WKMAT6CN3AJRPIIRYR2YM54TL2GDEMNQERFT",
            "450000.0",
            "12.5",
            "750000.0",
        ),
        (
            "USDC",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
            "yUSDC",
            "GDGTVWSM4MGS2T7Z7GVZE5SAEVLSWM5SGY5Q2EMUQWRMEV2RNYY3YFG6",
            "180000.0",
            "179500.0",
            "360000.0",
        ),
        (
            "XLM",
            "",
            "AQUA",
            "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
            "800000.0",
            "5000000.0",
            "420000.0",
        ),
    ];

    pool_configs
        .iter()
        .take(limit as usize)
        .enumerate()
        .map(
            |(i, (code_a, issuer_a, code_b, issuer_b, amt_a, amt_b, shares))| {
                let asset_a = if issuer_a.is_empty() {
                    "native".to_string()
                } else {
                    format!("{code_a}:{issuer_a}")
                };
                let asset_b = if issuer_b.is_empty() {
                    "native".to_string()
                } else {
                    format!("{code_b}:{issuer_b}")
                };

                HorizonLiquidityPool {
                    id: format!("pool_{:064x}", i + 1),
                    fee_bp: 30,
                    pool_type: "constant_product".to_string(),
                    total_trustlines: 100 + (i as u64 * 50),
                    total_shares: (*shares).to_string(),
                    reserves: vec![
                        HorizonPoolReserve {
                            asset: asset_a,
                            amount: (*amt_a).to_string(),
                        },
                        HorizonPoolReserve {
                            asset: asset_b,
                            amount: (*amt_b).to_string(),
                        },
                    ],
                    paging_token: Some(format!("pt_pool_{i}")),
                }
            },
        )
        .collect()
}

pub fn mock_assets(limit: u32) -> Vec<HorizonAsset> {
    let mut assets = Vec::new();
    let issues = [
        (
            "USDC",
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
        ),
        (
            "AQUA",
            "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
        ),
        (
            "yXLM",
            "GARDNV3Q7YGT4AKSDF25A9NTVAMQUD8UAKGHXONL6R2FMBXVGFZDFZEM",
        ),
        (
            "BTC",
            "GDPJALI4AZKUU2W426U5WKMAT6CN3AJRPIIRYR2YM54TL2GDEMNQERFT",
        ),
    ];

    for (i, (code, issuer)) in issues.iter().take(limit as usize).enumerate() {
        let base_trustlines = 10_000 - (i as i32 * 2_000);
        assets.push(HorizonAsset {
            asset_type: "credit_alphanum4".to_string(),
            asset_code: (*code).to_string(),
            asset_issuer: (*issuer).to_string(),
            num_claimable_balances: 0,
            num_liquidity_pools: 0,
            num_contracts: 0,
            accounts: AssetAccounts {
                authorized: base_trustlines,
                authorized_to_maintain_liabilities: 0,
                unauthorized: base_trustlines / 20,
            },
            claimable_balances_amount: "0.0".to_string(),
            liquidity_pools_amount: "0.0".to_string(),
            contracts_amount: "0.0".to_string(),
            balances: AssetBalances {
                authorized: format!("{}.0000000", base_trustlines * 1000),
                authorized_to_maintain_liabilities: "0.0".to_string(),
                unauthorized: "0.0".to_string(),
            },
            flags: AssetFlags {
                auth_required: false,
                auth_revocable: false,
                auth_immutable: false,
                auth_clawback_enabled: false,
            },
        });
    }
    assets
}
