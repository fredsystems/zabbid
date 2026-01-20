-- Create round_groups table
CREATE TABLE round_groups (
    round_group_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    editing_enabled TINYINT NOT NULL DEFAULT 1 CHECK(editing_enabled IN (0, 1)),
    UNIQUE (bid_year_id, name),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id)
) ENGINE=InnoDB;

-- Create rounds table
CREATE TABLE rounds (
    round_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    area_id BIGINT NOT NULL,
    round_group_id BIGINT NOT NULL,
    round_number INT NOT NULL,
    name VARCHAR(255) NOT NULL,
    slots_per_day INT NOT NULL CHECK(slots_per_day > 0),
    max_groups INT NOT NULL CHECK(max_groups > 0),
    max_total_hours INT NOT NULL CHECK(max_total_hours > 0),
    include_holidays TINYINT NOT NULL DEFAULT 0 CHECK(include_holidays IN (0, 1)),
    allow_overbid TINYINT NOT NULL DEFAULT 0 CHECK(allow_overbid IN (0, 1)),
    UNIQUE (area_id, round_number),
    FOREIGN KEY(area_id) REFERENCES areas(area_id),
    FOREIGN KEY(round_group_id) REFERENCES round_groups(round_group_id)
) ENGINE=InnoDB;
