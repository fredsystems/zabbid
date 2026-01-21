-- Phase 29B Semantic Correction Rollback: Revert to old schema

-- Step 1: Recreate rounds table with area_id
CREATE TABLE rounds_new (
    round_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    area_id INTEGER NOT NULL,
    round_group_id INTEGER NOT NULL,
    round_number INTEGER NOT NULL,
    name TEXT NOT NULL,
    slots_per_day INTEGER NOT NULL CHECK(slots_per_day > 0),
    max_groups INTEGER NOT NULL CHECK(max_groups > 0),
    max_total_hours INTEGER NOT NULL CHECK(max_total_hours > 0),
    include_holidays INTEGER NOT NULL DEFAULT 0 CHECK(include_holidays IN (0, 1)),
    allow_overbid INTEGER NOT NULL DEFAULT 0 CHECK(allow_overbid IN (0, 1)),
    UNIQUE (area_id, round_number),
    FOREIGN KEY(area_id) REFERENCES areas(area_id),
    FOREIGN KEY(round_group_id) REFERENCES round_groups(round_group_id)
);

-- Copy data back (Note: area_id will be NULL and must be manually fixed)
INSERT INTO rounds_new (
    round_id,
    round_group_id,
    round_number,
    name,
    slots_per_day,
    max_groups,
    max_total_hours,
    include_holidays,
    allow_overbid
)
SELECT
    round_id,
    round_group_id,
    round_number,
    name,
    slots_per_day,
    max_groups,
    max_total_hours,
    include_holidays,
    allow_overbid
FROM rounds;

-- Drop corrected rounds table
DROP TABLE rounds;

-- Rename to rounds
ALTER TABLE rounds_new RENAME TO rounds;

-- Step 2: Remove round_group_id from areas
-- SQLite doesn't support DROP COLUMN directly, so we recreate the table
CREATE TABLE areas_new (
    area_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_code TEXT NOT NULL,
    area_name TEXT,
    is_system_area INTEGER NOT NULL DEFAULT 0 CHECK(is_system_area IN (0, 1)),
    UNIQUE (bid_year_id, area_code),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id)
);

INSERT INTO areas_new (area_id, bid_year_id, area_code, area_name, is_system_area)
SELECT area_id, bid_year_id, area_code, area_name, is_system_area
FROM areas;

DROP TABLE areas;
ALTER TABLE areas_new RENAME TO areas;
