-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Canonical state tables (Phase 7, Phase 23A: canonical IDs)
CREATE TABLE bid_years (
    bid_year_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    year INTEGER NOT NULL UNIQUE,
    start_date TEXT NOT NULL,
    num_pay_periods INTEGER NOT NULL CHECK(num_pay_periods IN (26, 27)),
    is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0, 1)),
    expected_area_count INTEGER
);

CREATE TABLE areas (
    area_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_code TEXT NOT NULL,
    area_name TEXT,
    expected_user_count INTEGER,
    UNIQUE (bid_year_id, area_code),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id)
);

CREATE TABLE users (
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

CREATE INDEX idx_users_by_area ON users(bid_year_id, area_id);
CREATE INDEX idx_users_by_initials ON users(bid_year_id, area_id, initials);
