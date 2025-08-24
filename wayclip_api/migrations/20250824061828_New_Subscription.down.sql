DROP INDEX IF EXISTS idx_clips_user_id;
DROP TABLE IF EXISTS clips;

ALTER TYPE subscription_tier RENAME TO subscription_tier_new;
CREATE TYPE subscription_tier AS ENUM ('free', 'paid');
ALTER TABLE users
ALTER COLUMN tier TYPE subscription_tier
USING tier::text::subscription_tier;
DROP TYPE subscription_tier_new;
