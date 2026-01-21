-- Phase 29D: Rollback no_bid_reviewed flag addition
--
-- This migration removes the no_bid_reviewed column from the users table.

-- MySQL supports DROP COLUMN directly
ALTER TABLE users DROP COLUMN no_bid_reviewed;
