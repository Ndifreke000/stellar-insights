use crate::database::Database;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub title: String,
    pub description: Option<String>,
    pub proposal_type: Option<String>,
    pub target_contract: Option<String>,
    pub new_wasm_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProposalResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub proposal_type: String,
    pub target_contract: Option<String>,
    pub new_wasm_hash: Option<String>,
    pub status: String,
    pub created_by: String,
    pub on_chain_id: Option<i64>,
    pub voting_ends_at: Option<String>,
    pub finalized_at: Option<String>,
    pub executed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub votes_for: i64,
    pub votes_against: i64,
    pub votes_abstain: i64,
}

#[derive(Debug, Serialize)]
pub struct ProposalsListResponse {
    pub proposals: Vec<ProposalResponse>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct CastVoteRequest {
    pub choice: String,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VoteResponse {
    pub id: String,
    pub proposal_id: String,
    pub voter_address: String,
    pub choice: String,
    pub tx_hash: Option<String>,
    pub voted_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AddCommentRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub proposal_id: String,
    pub author_address: String,
    pub content: String,
    pub created_at: String,
}

pub struct GovernanceService {
    db: Arc<Database>,
}

impl GovernanceService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn create_proposal(
        &self,
        creator: &str,
        request: CreateProposalRequest,
    ) -> Result<ProposalResponse> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let proposal_type = request.proposal_type.unwrap_or_else(|| "contract_upgrade".to_string());

        sqlx::query(
            r#"
            INSERT INTO governance_proposals
            (id, title, description, proposal_type, target_contract, new_wasm_hash, status, created_by, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 'draft', ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&request.title)
        .bind(&request.description)
        .bind(&proposal_type)
        .bind(&request.target_contract)
        .bind(&request.new_wasm_hash)
        .bind(creator)
        .bind(&now)
        .bind(&now)
        .execute(self.db.pool())
        .await
        .context("Failed to create proposal")?;

        info!("Created governance proposal {} by {}", id, creator);

        Ok(ProposalResponse {
            id,
            title: request.title,
            description: request.description,
            proposal_type,
            target_contract: request.target_contract,
            new_wasm_hash: request.new_wasm_hash,
            status: "draft".to_string(),
            created_by: creator.to_string(),
            on_chain_id: None,
            voting_ends_at: None,
            finalized_at: None,
            executed_at: None,
            created_at: now.clone(),
            updated_at: now,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
        })
    }

    pub async fn activate_proposal(
        &self,
        proposal_id: &str,
        voting_duration_secs: i64,
    ) -> Result<ProposalResponse> {
        let now = Utc::now();
        let voting_ends_at = now + chrono::Duration::seconds(voting_duration_secs);
        let now_str = now.to_rfc3339();
        let voting_ends_str = voting_ends_at.to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE governance_proposals
            SET status = 'active', voting_ends_at = ?, updated_at = ?
            WHERE id = ? AND status = 'draft'
            "#,
        )
        .bind(&voting_ends_str)
        .bind(&now_str)
        .bind(proposal_id)
        .execute(self.db.pool())
        .await
        .context("Failed to activate proposal")?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Proposal not found or not in draft status"));
        }

        info!("Activated governance proposal {}", proposal_id);
        self.get_proposal(proposal_id).await
    }

    pub async fn list_proposals(
        &self,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<ProposalsListResponse> {
        let (rows, total) = if let Some(status) = status {
            let rows = sqlx::query(
                r#"
                SELECT p.*,
                    COALESCE(SUM(CASE WHEN v.choice = 'for' THEN 1 ELSE 0 END), 0) as votes_for,
                    COALESCE(SUM(CASE WHEN v.choice = 'against' THEN 1 ELSE 0 END), 0) as votes_against,
                    COALESCE(SUM(CASE WHEN v.choice = 'abstain' THEN 1 ELSE 0 END), 0) as votes_abstain
                FROM governance_proposals p
                LEFT JOIN governance_votes v ON p.id = v.proposal_id
                WHERE p.status = ?
                GROUP BY p.id
                ORDER BY p.created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db.pool())
            .await
            .context("Failed to list proposals")?;

            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM governance_proposals WHERE status = ?",
            )
            .bind(status)
            .fetch_one(self.db.pool())
            .await
            .context("Failed to count proposals")?;

            (rows, total)
        } else {
            let rows = sqlx::query(
                r#"
                SELECT p.*,
                    COALESCE(SUM(CASE WHEN v.choice = 'for' THEN 1 ELSE 0 END), 0) as votes_for,
                    COALESCE(SUM(CASE WHEN v.choice = 'against' THEN 1 ELSE 0 END), 0) as votes_against,
                    COALESCE(SUM(CASE WHEN v.choice = 'abstain' THEN 1 ELSE 0 END), 0) as votes_abstain
                FROM governance_proposals p
                LEFT JOIN governance_votes v ON p.id = v.proposal_id
                GROUP BY p.id
                ORDER BY p.created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(self.db.pool())
            .await
            .context("Failed to list proposals")?;

            let total: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM governance_proposals")
                    .fetch_one(self.db.pool())
                    .await
                    .context("Failed to count proposals")?;

            (rows, total)
        };

        let proposals = rows
            .iter()
            .map(|row| proposal_from_row(row))
            .collect::<Result<Vec<_>>>()?;

        Ok(ProposalsListResponse { proposals, total })
    }

    pub async fn get_proposal(&self, id: &str) -> Result<ProposalResponse> {
        let row = sqlx::query(
            r#"
            SELECT p.*,
                COALESCE(SUM(CASE WHEN v.choice = 'for' THEN 1 ELSE 0 END), 0) as votes_for,
                COALESCE(SUM(CASE WHEN v.choice = 'against' THEN 1 ELSE 0 END), 0) as votes_against,
                COALESCE(SUM(CASE WHEN v.choice = 'abstain' THEN 1 ELSE 0 END), 0) as votes_abstain
            FROM governance_proposals p
            LEFT JOIN governance_votes v ON p.id = v.proposal_id
            WHERE p.id = ?
            GROUP BY p.id
            "#,
        )
        .bind(id)
        .fetch_one(self.db.pool())
        .await
        .context("Proposal not found")?;

        proposal_from_row(&row)
    }

    pub async fn cast_vote(
        &self,
        proposal_id: &str,
        voter_address: &str,
        request: CastVoteRequest,
    ) -> Result<VoteResponse> {
        // Verify proposal is active
        let status: String = sqlx::query_scalar(
            "SELECT status FROM governance_proposals WHERE id = ?",
        )
        .bind(proposal_id)
        .fetch_one(self.db.pool())
        .await
        .context("Proposal not found")?;

        if status != "active" {
            return Err(anyhow!("Proposal is not active for voting"));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO governance_votes (id, proposal_id, voter_address, choice, tx_hash, voted_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(proposal_id)
        .bind(voter_address)
        .bind(&request.choice)
        .bind(&request.tx_hash)
        .bind(&now)
        .execute(self.db.pool())
        .await
        .context("Failed to cast vote (may have already voted)")?;

        info!(
            "Vote cast on proposal {} by {}: {}",
            proposal_id, voter_address, request.choice
        );

        Ok(VoteResponse {
            id,
            proposal_id: proposal_id.to_string(),
            voter_address: voter_address.to_string(),
            choice: request.choice,
            tx_hash: request.tx_hash,
            voted_at: now,
        })
    }

    pub async fn get_votes(&self, proposal_id: &str, limit: i64) -> Result<Vec<VoteResponse>> {
        let rows = sqlx::query(
            r#"
            SELECT id, proposal_id, voter_address, choice, tx_hash, voted_at
            FROM governance_votes
            WHERE proposal_id = ?
            ORDER BY voted_at DESC
            LIMIT ?
            "#,
        )
        .bind(proposal_id)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to fetch votes")?;

        let mut votes = Vec::new();
        for row in rows {
            votes.push(VoteResponse {
                id: row.try_get::<String, _>("id")?,
                proposal_id: row.try_get::<String, _>("proposal_id")?,
                voter_address: row.try_get::<String, _>("voter_address")?,
                choice: row.try_get::<String, _>("choice")?,
                tx_hash: row.try_get("tx_hash").ok(),
                voted_at: row.try_get::<String, _>("voted_at")?,
            });
        }

        Ok(votes)
    }

    pub async fn has_voted(&self, proposal_id: &str, voter_address: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM governance_votes WHERE proposal_id = ? AND voter_address = ?",
        )
        .bind(proposal_id)
        .bind(voter_address)
        .fetch_one(self.db.pool())
        .await
        .context("Failed to check vote status")?;

        Ok(count > 0)
    }

    pub async fn add_comment(
        &self,
        proposal_id: &str,
        author: &str,
        request: AddCommentRequest,
    ) -> Result<CommentResponse> {
        // Verify proposal exists
        let _: String = sqlx::query_scalar(
            "SELECT id FROM governance_proposals WHERE id = ?",
        )
        .bind(proposal_id)
        .fetch_one(self.db.pool())
        .await
        .context("Proposal not found")?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO governance_comments (id, proposal_id, author_address, content, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(proposal_id)
        .bind(author)
        .bind(&request.content)
        .bind(&now)
        .execute(self.db.pool())
        .await
        .context("Failed to add comment")?;

        info!("Comment added to proposal {} by {}", proposal_id, author);

        Ok(CommentResponse {
            id,
            proposal_id: proposal_id.to_string(),
            author_address: author.to_string(),
            content: request.content,
            created_at: now,
        })
    }

    pub async fn get_comments(
        &self,
        proposal_id: &str,
        limit: i64,
    ) -> Result<Vec<CommentResponse>> {
        let rows = sqlx::query(
            r#"
            SELECT id, proposal_id, author_address, content, created_at
            FROM governance_comments
            WHERE proposal_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(proposal_id)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to fetch comments")?;

        let mut comments = Vec::new();
        for row in rows {
            comments.push(CommentResponse {
                id: row.try_get::<String, _>("id")?,
                proposal_id: row.try_get::<String, _>("proposal_id")?,
                author_address: row.try_get::<String, _>("author_address")?,
                content: row.try_get::<String, _>("content")?,
                created_at: row.try_get::<String, _>("created_at")?,
            });
        }

        Ok(comments)
    }

    pub async fn update_status(&self, proposal_id: &str, status: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE governance_proposals SET status = ?, updated_at = ? WHERE id = ?",
        )
        .bind(status)
        .bind(&now)
        .bind(proposal_id)
        .execute(self.db.pool())
        .await
        .context("Failed to update proposal status")?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Proposal not found"));
        }

        info!("Updated proposal {} status to {}", proposal_id, status);
        Ok(())
    }
}

fn proposal_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ProposalResponse> {
    Ok(ProposalResponse {
        id: row.try_get::<String, _>("id")?,
        title: row.try_get::<String, _>("title")?,
        description: row.try_get("description").ok(),
        proposal_type: row
            .try_get::<String, _>("proposal_type")
            .unwrap_or_else(|_| "contract_upgrade".to_string()),
        target_contract: row.try_get("target_contract").ok(),
        new_wasm_hash: row.try_get("new_wasm_hash").ok(),
        status: row
            .try_get::<String, _>("status")
            .unwrap_or_else(|_| "draft".to_string()),
        created_by: row.try_get::<String, _>("created_by")?,
        on_chain_id: row.try_get("on_chain_id").ok(),
        voting_ends_at: row.try_get("voting_ends_at").ok(),
        finalized_at: row.try_get("finalized_at").ok(),
        executed_at: row.try_get("executed_at").ok(),
        created_at: row.try_get::<String, _>("created_at")?,
        updated_at: row.try_get::<String, _>("updated_at")?,
        votes_for: row.try_get::<i64, _>("votes_for").unwrap_or(0),
        votes_against: row.try_get::<i64, _>("votes_against").unwrap_or(0),
        votes_abstain: row.try_get::<i64, _>("votes_abstain").unwrap_or(0),
    })
}
