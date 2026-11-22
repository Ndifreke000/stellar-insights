-- contract-phase: drop tables added for anchor health scores, governance reconciliation, and stage latencies
DROP INDEX IF EXISTS idx_anchor_stage_latencies_anchor;
DROP TABLE IF EXISTS anchor_stage_latencies;
DROP INDEX IF EXISTS idx_gov_reconciliation_status;
DROP TABLE IF EXISTS governance_reconciliation_discrepancies;
DROP INDEX IF EXISTS idx_governance_chain_events_proposal;
DROP TABLE IF EXISTS governance_chain_events;
DROP INDEX IF EXISTS idx_anchor_health_scores_version;
DROP INDEX IF EXISTS idx_anchor_health_scores_anchor;
DROP TABLE IF EXISTS anchor_health_scores;
