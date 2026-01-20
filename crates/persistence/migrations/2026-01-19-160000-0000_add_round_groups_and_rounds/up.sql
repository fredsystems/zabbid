-- Create round_groups table
CREATE TABLE round_groups (
    round_group_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    editing_enabled INTEGER NOT NULL DEFAULT 1 CHECK(editing_enabled IN (0, 1)),
    UNIQUE (bid_year_id, name),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id)
);

-- Create rounds table
CREATE TABLE rounds (
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
