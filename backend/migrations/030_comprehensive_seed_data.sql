-- Comprehensive seed data for development and testing
-- This migration provides realistic test data for all major entities

-- ============================================================================
-- ANCHORS - Primary payment service providers
-- ============================================================================

INSERT OR IGNORE INTO anchors (id, name, stellar_account, home_domain, total_transactions, successful_transactions, failed_transactions, total_volume_usd, avg_settlement_time_ms, reliability_score, status, created_at, updated_at)
VALUES 
-- Green status anchors (highly reliable)
('anchor-circle-001', 'Circle', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 'circle.com', 125000, 124125, 875, 15500000.00, 1200, 99.3, 'green', datetime('now', '-90 days'), datetime('now')),
('anchor-moneygram-001', 'MoneyGram Access', 'GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RIAA', 'moneygram.com', 98500, 97415, 1085, 9800000.00, 1500, 98.9, 'green', datetime('now', '-60 days'), datetime('now')),
('anchor-stellar-001', 'Stellar Development Foundation', 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN7', 'stellar.org', 250000, 248750, 1250, 50000000.00, 800, 99.5, 'green', datetime('now', '-120 days'), datetime('now')),

-- Yellow status anchors (caution - moderate reliability)
('anchor-anchorusd-001', 'AnchorUSD', 'GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLAA', 'anchorusd.com', 45000, 42750, 2250, 7500000.00, 3500, 95.0, 'yellow', datetime('now', '-45 days'), datetime('now')),
('anchor-vibrant-001', 'Vibrant', 'GBHFGY3ZNEJWLNO4LBUKLYOCEK4V7ENEBJGPRHHX7JU47GWHBREH37UR', 'vibrantapp.com', 32000, 30400, 1600, 4200000.00, 4200, 94.8, 'yellow', datetime('now', '-30 days'), datetime('now')),

-- Red status anchors (unreliable - needs attention)
('anchor-testnet-001', 'TestNet Anchor', 'GCTESTANCHOR12345678901234567890123456789012345', 'testanchor.io', 8000, 6400, 1600, 800000.00, 8500, 80.0, 'red', datetime('now', '-15 days'), datetime('now')),
('anchor-legacy-001', 'Legacy Bridge', 'GLEGACYBRIDGE1234567890123456789012345678901234', 'legacy.example.com', 12000, 9600, 2400, 1200000.00, 6800, 80.0, 'red', datetime('now', '-7 days'), datetime('now'));

-- ============================================================================
-- ASSETS - Issued assets by anchors
-- ============================================================================

INSERT OR IGNORE INTO assets (id, anchor_id, asset_code, asset_issuer, total_supply, num_holders, blockchain_chain, created_at, updated_at)
VALUES 
-- Circle assets
('asset-usdc-001', 'anchor-circle-001', 'USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 5000000000, 250000, 'stellar', datetime('now', '-90 days'), datetime('now')),
('asset-eurc-001', 'anchor-circle-001', 'EURC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 2000000000, 80000, 'stellar', datetime('now', '-90 days'), datetime('now')),
('asset-gbpd-001', 'anchor-circle-001', 'GBPD', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 1500000000, 60000, 'stellar', datetime('now', '-85 days'), datetime('now')),

-- MoneyGram assets
('asset-mgi-001', 'anchor-moneygram-001', 'MGI', 'GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RIAA', 800000000, 120000, 'stellar', datetime('now', '-60 days'), datetime('now')),
('asset-mgusd-001', 'anchor-moneygram-001', 'MGUSD', 'GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RIAA', 600000000, 90000, 'stellar', datetime('now', '-55 days'), datetime('now')),

-- Stellar Foundation assets
('asset-xlm-001', 'anchor-stellar-001', 'XLM', 'native', 50000000000, 5000000, 'stellar', datetime('now', '-120 days'), datetime('now')),

-- AnchorUSD assets
('asset-ausd-001', 'anchor-anchorusd-001', 'AUSD', 'GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLAA', 300000000, 50000, 'stellar', datetime('now', '-45 days'), datetime('now')),

-- Vibrant assets
('asset-velo-001', 'anchor-vibrant-001', 'VELO', 'GBHFGY3ZNEJWLNO4LBUKLYOCEK4V7ENEBJGPRHHX7JU47GWHBREH37UR', 1000000000, 150000, 'stellar', datetime('now', '-30 days'), datetime('now')),

-- TestNet assets
('asset-test-001', 'anchor-testnet-001', 'TEST', 'GCTESTANCHOR12345678901234567890123456789012345', 100000000, 5000, 'stellar', datetime('now', '-15 days'), datetime('now'));

-- ============================================================================
-- CORRIDORS - Asset trading pairs and liquidity routes
-- ============================================================================

INSERT OR IGNORE INTO corridors (id, source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer, reliability_score, status, source_code, destination_code, created_at, updated_at)
VALUES 
-- USDC corridors (high volume)
('corridor-usdc-eurc-001', 'USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 'EURC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 99.2, 'active', 'USDC', 'EURC', datetime('now', '-90 days'), datetime('now')),
('corridor-usdc-gbpd-001', 'USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 'GBPD', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 98.8, 'active', 'USDC', 'GBPD', datetime('now', '-85 days'), datetime('now')),
('corridor-usdc-mgi-001', 'USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 'MGI', 'GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RIAA', 97.5, 'active', 'USDC', 'MGI', datetime('now', '-80 days'), datetime('now')),

-- XLM corridors (native asset)
('corridor-xlm-usdc-001', 'XLM', 'native', 'USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 99.5, 'active', 'XLM', 'USDC', datetime('now', '-120 days'), datetime('now')),
('corridor-xlm-mgi-001', 'XLM', 'native', 'MGI', 'GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RIAA', 98.2, 'active', 'XLM', 'MGI', datetime('now', '-100 days'), datetime('now')),

-- MoneyGram corridors
('corridor-mgi-mgusd-001', 'MGI', 'GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RIAA', 'MGUSD', 'GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RIAA', 96.5, 'active', 'MGI', 'MGUSD', datetime('now', '-60 days'), datetime('now')),

-- Vibrant corridors
('corridor-velo-usdc-001', 'VELO', 'GBHFGY3ZNEJWLNO4LBUKLYOCEK4V7ENEBJGPRHHX7JU47GWHBREH37UR', 'USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 94.2, 'active', 'VELO', 'USDC', datetime('now', '-30 days'), datetime('now')),

-- Inactive/degraded corridors
('corridor-test-usdc-001', 'TEST', 'GCTESTANCHOR12345678901234567890123456789012345', 'USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZAA', 75.0, 'degraded', 'TEST', 'USDC', datetime('now', '-15 days'), datetime('now'));

-- ============================================================================
-- ANCHOR METRICS HISTORY - Time-series performance data
-- ============================================================================

INSERT OR IGNORE INTO anchor_metrics_history (id, anchor_id, timestamp, success_rate, failure_rate, reliability_score, total_transactions, successful_transactions, failed_transactions, avg_settlement_time_ms, volume_usd, created_at)
VALUES 
-- Circle metrics (last 30 days)
('metric-circle-001', 'anchor-circle-001', datetime('now', '-30 days'), 99.5, 0.5, 99.3, 4000, 3980, 20, 1200, 500000.00, datetime('now')),
('metric-circle-002', 'anchor-circle-001', datetime('now', '-20 days'), 99.4, 0.6, 99.2, 4200, 4177, 23, 1250, 520000.00, datetime('now')),
('metric-circle-003', 'anchor-circle-001', datetime('now', '-10 days'), 99.6, 0.4, 99.4, 4100, 4084, 16, 1180, 510000.00, datetime('now')),

-- MoneyGram metrics
('metric-moneygram-001', 'anchor-moneygram-001', datetime('now', '-30 days'), 98.8, 1.2, 98.7, 3200, 3162, 38, 1600, 320000.00, datetime('now')),
('metric-moneygram-002', 'anchor-moneygram-001', datetime('now', '-15 days'), 99.0, 1.0, 98.9, 3300, 3267, 33, 1550, 330000.00, datetime('now')),

-- Stellar metrics
('metric-stellar-001', 'anchor-stellar-001', datetime('now', '-30 days'), 99.6, 0.4, 99.5, 8000, 7968, 32, 850, 1600000.00, datetime('now')),
('metric-stellar-002', 'anchor-stellar-001', datetime('now', '-15 days'), 99.7, 0.3, 99.6, 8200, 8170, 30, 800, 1650000.00, datetime('now')),

-- AnchorUSD metrics (yellow status)
('metric-anchorusd-001', 'anchor-anchorusd-001', datetime('now', '-30 days'), 94.5, 5.5, 94.8, 1500, 1418, 82, 3500, 250000.00, datetime('now')),
('metric-anchorusd-002', 'anchor-anchorusd-001', datetime('now', '-15 days'), 95.2, 4.8, 95.1, 1600, 1523, 77, 3400, 260000.00, datetime('now')),

-- Vibrant metrics (yellow status)
('metric-vibrant-001', 'anchor-vibrant-001', datetime('now', '-30 days'), 94.0, 6.0, 94.5, 1000, 940, 60, 4300, 140000.00, datetime('now')),
('metric-vibrant-002', 'anchor-vibrant-001', datetime('now', '-15 days'), 95.0, 5.0, 95.1, 1100, 1045, 55, 4100, 150000.00, datetime('now')),

-- TestNet metrics (red status)
('metric-testnet-001', 'anchor-testnet-001', datetime('now', '-30 days'), 75.0, 25.0, 75.0, 300, 225, 75, 8500, 50000.00, datetime('now')),
('metric-testnet-002', 'anchor-testnet-001', datetime('now', '-15 days'), 80.0, 20.0, 80.0, 350, 280, 70, 8200, 55000.00, datetime('now'));

-- ============================================================================
-- METRICS - Generic metric tracking
-- ============================================================================

INSERT OR IGNORE INTO metrics (id, name, value, entity_id, entity_type, timestamp, created_at)
VALUES 
-- Anchor metrics
('metric-circle-success-rate', 'transaction_success_rate', 99.3, 'anchor-circle-001', 'anchor', datetime('now'), datetime('now')),
('metric-circle-volume', 'daily_volume_usd', 500000.00, 'anchor-circle-001', 'anchor', datetime('now'), datetime('now')),
('metric-moneygram-success-rate', 'transaction_success_rate', 98.9, 'anchor-moneygram-001', 'anchor', datetime('now'), datetime('now')),
('metric-moneygram-volume', 'daily_volume_usd', 320000.00, 'anchor-moneygram-001', 'anchor', datetime('now'), datetime('now')),

-- Corridor metrics
('metric-usdc-eurc-volume', 'daily_volume_usd', 1200000.00, 'corridor-usdc-eurc-001', 'corridor', datetime('now'), datetime('now')),
('metric-usdc-eurc-success', 'transaction_success_rate', 99.2, 'corridor-usdc-eurc-001', 'corridor', datetime('now'), datetime('now')),
('metric-xlm-usdc-volume', 'daily_volume_usd', 2500000.00, 'corridor-xlm-usdc-001', 'corridor', datetime('now'), datetime('now')),
('metric-xlm-usdc-success', 'transaction_success_rate', 99.5, 'corridor-xlm-usdc-001', 'corridor', datetime('now'), datetime('now'));

-- ============================================================================
-- USERS - Test users for authentication
-- ============================================================================

INSERT OR IGNORE INTO users (id, username, password_hash, created_at, updated_at)
VALUES 
('user-admin-001', 'admin', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5YmMxSUaqvMJi', datetime('now', '-90 days'), datetime('now')),
('user-analyst-001', 'analyst', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5YmMxSUaqvMJi', datetime('now', '-60 days'), datetime('now')),
('user-developer-001', 'developer', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5YmMxSUaqvMJi', datetime('now', '-30 days'), datetime('now')),
('user-viewer-001', 'viewer', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5YmMxSUaqvMJi', datetime('now', '-15 days'), datetime('now'));

-- ============================================================================
-- API KEYS - Test API keys for development
-- ============================================================================

INSERT OR IGNORE INTO api_keys (id, name, key_prefix, key_hash, wallet_address, scopes, status, created_at, last_used_at, expires_at, revoked_at)
VALUES 
('key-admin-001', 'Admin Key', 'sk_admin_', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 'GADMIN123456789012345678901234567890123456789012345', 'read,write,admin', 'active', datetime('now', '-90 days'), datetime('now'), NULL, NULL),
('key-analyst-001', 'Analyst Key', 'sk_analyst_', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b856', 'GANALYST12345678901234567890123456789012345678901234', 'read', 'active', datetime('now', '-60 days'), datetime('now'), NULL, NULL),
('key-dev-001', 'Development Key', 'sk_dev_', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b857', 'GDEV1234567890123456789012345678901234567890123456789', 'read,write', 'active', datetime('now', '-30 days'), datetime('now'), NULL, NULL),
('key-expired-001', 'Expired Key', 'sk_expired_', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b858', 'GEXPIRED123456789012345678901234567890123456789012345', 'read', 'active', datetime('now', '-180 days'), NULL, datetime('now', '-30 days'), NULL),
('key-revoked-001', 'Revoked Key', 'sk_revoked_', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b859', 'GREVOKED123456789012345678901234567890123456789012345', 'read,write', 'revoked', datetime('now', '-120 days'), datetime('now', '-60 days'), NULL, datetime('now', '-30 days'));

-- ============================================================================
-- GOVERNANCE - Proposals and voting
-- ============================================================================

INSERT OR IGNORE INTO governance_proposals (id, title, description, proposal_type, target_contract, new_wasm_hash, status, created_by, on_chain_id, voting_ends_at, finalized_at, executed_at, created_at, updated_at)
VALUES 
('proposal-001', 'Upgrade Core Contract', 'Upgrade the core contract to v2.0 with performance improvements', 'contract_upgrade', 'core_contract', 'abc123def456', 'active', 'GPROPOSER1234567890123456789012345678901234567890123', 1, datetime('now', '+7 days'), NULL, NULL, datetime('now', '-7 days'), datetime('now')),
('proposal-002', 'Increase Fee Threshold', 'Increase the minimum fee threshold from 0.1 XLM to 0.5 XLM', 'parameter_change', NULL, NULL, 'active', 'GPROPOSER1234567890123456789012345678901234567890123', 2, datetime('now', '+5 days'), NULL, NULL, datetime('now', '-5 days'), datetime('now')),
('proposal-003', 'Add New Anchor', 'Add Circle as an approved anchor provider', 'anchor_approval', NULL, NULL, 'passed', 'GPROPOSER1234567890123456789012345678901234567890123', 3, datetime('now', '-10 days'), datetime('now', '-5 days'), datetime('now', '-2 days'), datetime('now', '-20 days'), datetime('now', '-2 days')),
('proposal-004', 'Deprecate Legacy Bridge', 'Deprecate the legacy bridge contract', 'deprecation', 'legacy_bridge', NULL, 'rejected', 'GPROPOSER1234567890123456789012345678901234567890123', 4, datetime('now', '-30 days'), datetime('now', '-25 days'), NULL, datetime('now', '-40 days'), datetime('now', '-25 days'));

-- ============================================================================
-- GOVERNANCE VOTES - Voting records
-- ============================================================================

INSERT OR IGNORE INTO governance_votes (id, proposal_id, voter_address, choice, tx_hash, voted_at)
VALUES 
-- Proposal 1 votes
('vote-001', 'proposal-001', 'GVOTER1111111111111111111111111111111111111111111111', 'yes', 'tx_hash_001', datetime('now', '-5 days')),
('vote-002', 'proposal-001', 'GVOTER2222222222222222222222222222222222222222222222', 'yes', 'tx_hash_002', datetime('now', '-4 days')),
('vote-003', 'proposal-001', 'GVOTER3333333333333333333333333333333333333333333333', 'no', 'tx_hash_003', datetime('now', '-3 days')),

-- Proposal 2 votes
('vote-004', 'proposal-002', 'GVOTER1111111111111111111111111111111111111111111111', 'yes', 'tx_hash_004', datetime('now', '-3 days')),
('vote-005', 'proposal-002', 'GVOTER2222222222222222222222222222222222222222222222', 'abstain', 'tx_hash_005', datetime('now', '-2 days')),

-- Proposal 3 votes (passed)
('vote-006', 'proposal-003', 'GVOTER1111111111111111111111111111111111111111111111', 'yes', 'tx_hash_006', datetime('now', '-15 days')),
('vote-007', 'proposal-003', 'GVOTER2222222222222222222222222222222222222222222222', 'yes', 'tx_hash_007', datetime('now', '-14 days')),
('vote-008', 'proposal-003', 'GVOTER3333333333333333333333333333333333333333333333', 'yes', 'tx_hash_008', datetime('now', '-13 days')),

-- Proposal 4 votes (rejected)
('vote-009', 'proposal-004', 'GVOTER1111111111111111111111111111111111111111111111', 'no', 'tx_hash_009', datetime('now', '-35 days')),
('vote-010', 'proposal-004', 'GVOTER2222222222222222222222222222222222222222222222', 'no', 'tx_hash_010', datetime('now', '-34 days')),
('vote-011', 'proposal-004', 'GVOTER3333333333333333333333333333333333333333333333', 'no', 'tx_hash_011', datetime('now', '-33 days'));

-- ============================================================================
-- GOVERNANCE COMMENTS - Discussion on proposals
-- ============================================================================

INSERT OR IGNORE INTO governance_comments (id, proposal_id, author_address, content, created_at)
VALUES 
('comment-001', 'proposal-001', 'GCOMMENT1111111111111111111111111111111111111111111', 'Great improvement! Looking forward to the performance gains.', datetime('now', '-6 days')),
('comment-002', 'proposal-001', 'GCOMMENT2222222222222222222222222222222222222222222', 'Has this been audited by a third party?', datetime('now', '-5 days')),
('comment-003', 'proposal-001', 'GCOMMENT3333333333333333333333333333333333333333333', 'Yes, audit report is available at https://example.com/audit', datetime('now', '-4 days')),
('comment-004', 'proposal-002', 'GCOMMENT1111111111111111111111111111111111111111111', 'This might impact small transactions. Need to discuss impact.', datetime('now', '-4 days')),
('comment-005', 'proposal-003', 'GCOMMENT2222222222222222222222222222222222222222222', 'Circle is a trusted partner. Excited to have them on board!', datetime('now', '-18 days'));
