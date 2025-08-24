-- Add up migration script here

ALTER TYPE subscription_tier RENAME TO subscription_tier_old;

CREATE TYPE subscription_tier AS ENUM ('free', 'tier1', 'tier2', 'tier3');

ALTER TABLE users ALTER COLUMN tier DROP DEFAULT;

ALTER TABLE users
ALTER COLUMN tier TYPE subscription_tier
USING tier::text::subscription_tier;

ALTER TABLE users ALTER COLUMN tier SET DEFAULT 'free'::subscription_tier;

DROP TYPE subscription_tier_old;

CREATE TABLE clips (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    public_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_clips_user_id ON clips(user_id);
