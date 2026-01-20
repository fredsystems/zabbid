-- Add foreign key constraint for areas.round_group_id
-- SQLite requires recreating the table to add foreign key constraints

-- Create new areas table with foreign key constraint
CREATE TABLE areas_new (
    area_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_code TEXT NOT NULL,
    area_name TEXT,
    expected_user_count INTEGER,
    is_system_area INTEGER NOT NULL DEFAULT 0 CHECK(is_system_area IN (0, 1)),
    round_group_id INTEGER,
    UNIQUE (bid_year_id, area_code),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(round_group_id) REFERENCES round_groups(round_group_id)
);

-- Copy data from old areas table
INSERT INTO areas_new (
    area_id,
    bid_year_id,
    area_code,
    area_name,
    expected_user_count,
    is_system_area,
    round_group_id
)
SELECT
    area_id,
    bid_year_id,
    area_code,
    area_name,
    expected_user_count,
    is_system_area,
    round_group_id
FROM areas;

-- Drop old areas table
DROP TABLE areas;

-- Rename new areas table
ALTER TABLE areas_new RENAME TO areas;
