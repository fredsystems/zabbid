-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 25A: Remove lifecycle_state from bid_years
-- SQLite does not support DROP COLUMN directly, so we must recreate the table

-- Create new table without lifecycle_state
CREATE TABLE bid_years_new (
    bid_year_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    year INTEGER NOT NULL UNIQUE,
    start_date TEXT NOT NULL,
    num_pay_periods INTEGER NOT NULL CHECK(num_pay_periods IN (26, 27)),
    is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0, 1)),
    expected_area_count INTEGER
);

-- Copy data from old table
INSERT INTO bid_years_new (bid_year_id, year, start_date, num_pay_periods, is_active, expected_area_count)
SELECT bid_year_id, year, start_date, num_pay_periods, is_active, expected_area_count
FROM bid_years;

-- Drop old table
DROP TABLE bid_years;

-- Rename new table to original name
ALTER TABLE bid_years_new RENAME TO bid_years;
