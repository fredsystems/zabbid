-- Phase 29B Semantic Correction: Areas reference round groups, rounds belong to round groups only

-- Step 1: Add round_group_id to areas table
ALTER TABLE areas ADD COLUMN round_group_id INTEGER;

-- Step 2: Add foreign key constraint (SQLite doesn't support ALTER TABLE ADD CONSTRAINT,
-- so we document the constraint here; it will be enforced by Diesel schema)
-- FOREIGN KEY(round_group_id) REFERENCES round_groups(round_group_id)

-- Step 3: Recreate rounds table without area_id
-- SQLite doesn't support DROP COLUMN, so we need to recreate the table

-- Create new rounds table with correct schema
CREATE TABLE rounds_new (
    round_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    round_group_id INTEGER NOT NULL,
    round_number INTEGER NOT NULL,
    name TEXT NOT NULL,
    slots_per_day INTEGER NOT NULL CHECK(slots_per_day > 0),
    max_groups INTEGER NOT NULL CHECK(max_groups > 0),
    max_total_hours INTEGER NOT NULL CHECK(max_total_hours > 0),
    include_holidays INTEGER NOT NULL DEFAULT 0 CHECK(include_holidays IN (0, 1)),
    allow_overbid INTEGER NOT NULL DEFAULT 0 CHECK(allow_overbid IN (0, 1)),
    UNIQUE (round_group_id, round_number),
    FOREIGN KEY(round_group_id) REFERENCES round_groups(round_group_id)
);

-- Copy data from old rounds table to new rounds table
-- Note: area_id is dropped; round_group_id already exists and is preserved
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

-- Drop old rounds table
DROP TABLE rounds;

-- Rename new rounds table to rounds
ALTER TABLE rounds_new RENAME TO rounds;
