-- Drop indexes first
DROP INDEX idx_bid_status_history_audit_event ON bid_status_history;
DROP INDEX idx_bid_status_history_bid_status ON bid_status_history;
DROP INDEX idx_bid_status_round ON bid_status;
DROP INDEX idx_bid_status_user ON bid_status;
DROP INDEX idx_bid_status_bid_year_area ON bid_status;

-- Drop tables (history first due to foreign key)
DROP TABLE IF EXISTS bid_status_history;
DROP TABLE IF EXISTS bid_status;
