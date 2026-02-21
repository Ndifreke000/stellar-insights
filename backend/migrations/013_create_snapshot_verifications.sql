-- Create snapshot verifications table for tracking user verification attempts
CREATE TABLE IF NOT EXISTS snapshot_verifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    epoch INTEGER NOT NULL,
    submitted_hash TEXT NOT NULL,
    expected_hash TEXT NOT NULL,
    is_match BOOLEAN NOT NULL,
    reward_points INTEGER DEFAULT 0,
    verified_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
);

-- Create user rewards table for tracking accumulated rewards
CREATE TABLE IF NOT EXISTS user_rewards (
    user_id TEXT PRIMARY KEY,
    total_points INTEGER DEFAULT 0,
    successful_verifications INTEGER DEFAULT 0,
    failed_verifications INTEGER DEFAULT 0,
    last_verification_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_verifications_user ON snapshot_verifications(user_id);
CREATE INDEX IF NOT EXISTS idx_verifications_snapshot ON snapshot_verifications(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_verifications_epoch ON snapshot_verifications(epoch DESC);
CREATE INDEX IF NOT EXISTS idx_verifications_verified_at ON snapshot_verifications(verified_at DESC);
CREATE INDEX IF NOT EXISTS idx_user_rewards_points ON user_rewards(total_points DESC);

-- Create leaderboard view for easy querying
CREATE VIEW IF NOT EXISTS verification_leaderboard AS
SELECT 
    u.id,
    u.username,
    ur.total_points,
    ur.successful_verifications,
    ur.failed_verifications,
    CAST(ur.successful_verifications AS REAL) / 
        NULLIF(ur.successful_verifications + ur.failed_verifications, 0) * 100 AS success_rate,
    ur.last_verification_at
FROM users u
INNER JOIN user_rewards ur ON u.id = ur.user_id
ORDER BY ur.total_points DESC;
