-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 29A: Revert user participation flags

-- SQLite does not support DROP COLUMN directly.
-- To remove columns, we must recreate the table.

-- Create new users table without the participation flag columns
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
    UNIQUE (bid_year_id, area_id, initials),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id)
);

-- Copy data from old table (excluding the participation flag columns)
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
    lottery_value
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
    lottery_value
FROM users;

-- Drop old table
DROP TABLE users;

-- Rename new table to users
ALTER TABLE users_new RENAME TO users;

-- Recreate indices
CREATE INDEX idx_users_by_area ON users(bid_year_id, area_id);
CREATE INDEX idx_users_by_initials ON users(bid_year_id, area_id, initials);
