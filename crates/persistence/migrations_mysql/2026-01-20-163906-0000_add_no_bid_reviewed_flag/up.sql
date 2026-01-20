-- Phase 29D: Add no_bid_reviewed flag to users table
--
-- Purpose: Track whether users in the "No Bid" system area have been reviewed.
--
-- Review means:
--   - User was moved to a non-system area, OR
--   - User was explicitly confirmed to remain in No Bid
--
-- Default: false (0)
-- This flag is used by readiness evaluation to ensure all No Bid users are reviewed
-- before a bid year can be confirmed ready to bid.

ALTER TABLE users ADD COLUMN no_bid_reviewed TINYINT NOT NULL DEFAULT 0 CHECK(no_bid_reviewed IN (0, 1));
