-- Add round_id to bid_windows table to make windows per-round instead of per-user
-- This corrects the Phase 29E schema to match the intended design

-- MySQL doesn't support renaming with ALTER TABLE in the same way as SQLite
-- We need to recreate the table with the correct schema

-- Step 1: Drop existing table (assuming empty or test data only)
DROP TABLE IF EXISTS bid_windows;

-- Step 2: Create table with correct schema including round_id
CREATE TABLE bid_windows (
    bid_window_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    area_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    round_id BIGINT NOT NULL,
    window_start_datetime TEXT NOT NULL,
    window_end_datetime TEXT NOT NULL,
    UNIQUE KEY unique_bid_window (bid_year_id, area_id, user_id, round_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (area_id) REFERENCES areas(area_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (round_id) REFERENCES rounds(round_id)
) ENGINE=InnoDB;

-- Step 3: Create indexes for common queries
CREATE INDEX idx_bid_windows_bid_year_area ON bid_windows(bid_year_id, area_id);
CREATE INDEX idx_bid_windows_user ON bid_windows(user_id);
CREATE INDEX idx_bid_windows_round ON bid_windows(round_id);
