-- Drop indexes first
DROP INDEX IF EXISTS idx_bid_status_history_audit_event;
DROP INDEX IF EXISTS idx_bid_status_history_bid_status;
DROP INDEX IF EXISTS idx_bid_status_round;
DROP INDEX IF EXISTS idx_bid_status_user;
DROP INDEX IF EXISTS idx_bid_status_bid_year_area;

-- Drop tables (history first due to foreign key)
DROP TABLE IF EXISTS bid_status_history;
DROP TABLE IF EXISTS bid_status;
