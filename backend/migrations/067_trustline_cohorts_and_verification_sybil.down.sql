-- contract-phase: drop tables added for trustline cohorts and verification sybil resistance
DROP INDEX IF EXISTS idx_verification_review_status;
DROP TABLE IF EXISTS verification_review_queue;
DROP TABLE IF EXISTS cluster_reward_caps;
DROP INDEX IF EXISTS idx_verifier_clusters_cluster;
DROP TABLE IF EXISTS verifier_account_clusters;
DROP INDEX IF EXISTS idx_snapshot_verifications_epoch_user;
DROP TABLE IF EXISTS trustline_spam_accounts;
DROP INDEX IF EXISTS idx_trustline_events_asset_account;
DROP TABLE IF EXISTS trustline_events;
