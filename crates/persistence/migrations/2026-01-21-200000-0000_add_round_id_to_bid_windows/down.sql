-- Revert bid_windows table to original schema without round_id

-- Step 1: Create old table structure
CREATE TABLE bid_windows_old (
    bid_window_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    window_start_datetime TEXT NOT NULL,
    window_end_datetime TEXT NOT NULL,
    UNIQUE (bid_year_id, area_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (area_id) REFERENCES areas(area_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

-- Step 2: Cannot reliably migrate data back (multiple rounds collapse to single user record)
-- Assuming rollback of schema correction means discarding per-round data

-- Step 3: Drop new table
DROP TABLE bid_windows;

-- Step 4: Rename old table
ALTER TABLE bid_windows_old RENAME TO bid_windows;
