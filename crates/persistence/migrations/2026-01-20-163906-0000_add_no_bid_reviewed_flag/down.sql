-- Phase 29D: Rollback no_bid_reviewed flag addition
--
-- This migration removes the no_bid_reviewed column from the users table.

-- SQLite does not support DROP COLUMN directly in older versions.
-- We must recreate the table without the column.

-- Step 1: Create new users table without no_bid_reviewed
CREATE TABLE users_new (
    user_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    initials TEXT NOT NULL,
    name TEXT NOT NULL,
    user_type TEXT NOT NULL,
    crew INTEGER,
    cumulative_natca_bu_date TEXT NOT NULL,
    natca_bu_date TEXT NOT NULL,
    eod_faa_date TEXT NOT NULL,
    service_computation_date TEXT NOT NULL,
    lottery_value INTEGER,
    excluded_from_bidding INTEGER NOT NULL DEFAULT 0 CHECK(excluded_from_bidding IN (0, 1)),
    excluded_from_leave_calculation INTEGER NOT NULL DEFAULT 0 CHECK(excluded_from_leave_calculation IN (0, 1)),
    UNIQUE (bid_year_id, area_id, initials),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id)
);

-- Step 2: Copy data from old table
INSERT INTO users_new (
    user_id,
    bid_year_id,
    area_id,
    initials,
    name,
    user_type,
    crew,
    cumulative_natca_bu_date,
    natca_bu_date,
    eod_faa_date,
    service_computation_date,
    lottery_value,
    excluded_from_bidding,
    excluded_from_leave_calculation
)
SELECT
    user_id,
    bid_year_id,
    area_id,
    initials,
    name,
    user_type,
    crew,
    cumulative_natca_bu_date,
    natca_bu_date,
    eod_faa_date,
    service_computation_date,
    lottery_value,
    excluded_from_bidding,
    excluded_from_leave_calculation
FROM users;

-- Step 3: Drop old table
DROP TABLE users;

-- Step 4: Rename new table to users
ALTER TABLE users_new RENAME TO users;

-- Step 5: Recreate indexes
CREATE INDEX idx_users_by_area ON users(bid_year_id, area_id);
CREATE INDEX idx_users_by_initials ON users(bid_year_id, area_id, initials);
