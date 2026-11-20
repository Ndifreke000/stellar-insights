-- Create fence_counter table for monotonically increasing fencing tokens
CREATE TABLE IF NOT EXISTS fence_counter (
    role TEXT PRIMARY KEY,
    counter INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT (now() AT TIME ZONE 'utc')
);

-- Create index for quick lookups
CREATE INDEX IF NOT EXISTS idx_fence_counter_role ON fence_counter(role);
