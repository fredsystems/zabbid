-- Add round_id to bid_windows table to make windows per-round instead of per-user
-- This corrects the Phase 29E schema to match the intended design

-- Step 1: Create new table with correct schema
CREATE TABLE bid_windows_new (
    bid_window_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    round_id INTEGER NOT NULL,
    window_start_datetime TEXT NOT NULL,
    window_end_datetime TEXT NOT NULL,
    UNIQUE (bid_year_id, area_id, user_id, round_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (area_id) REFERENCES areas(area_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (round_id) REFERENCES rounds(round_id)
);

-- Step 2: Copy existing data (if any exists, it won't have round_id so we can't migrate it)
-- Since this is a schema correction and Phase 29 is not yet in production,
-- we assume the table is empty or contains only test data that should be discarded

-- Step 3: Drop old table
DROP TABLE bid_windows;

-- Step 4: Rename new table
ALTER TABLE bid_windows_new RENAME TO bid_windows;

-- Step 5: Create indexes for common queries
CREATE INDEX idx_bid_windows_bid_year_area ON bid_windows(bid_year_id, area_id);
CREATE INDEX idx_bid_windows_user ON bid_windows(user_id);
CREATE INDEX idx_bid_windows_round ON bid_windows(round_id);
