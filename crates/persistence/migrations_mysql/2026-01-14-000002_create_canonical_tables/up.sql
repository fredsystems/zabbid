-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Canonical state tables (Phase 7, Phase 23A: canonical IDs) - MySQL/MariaDB version
CREATE TABLE bid_years (
    bid_year_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    year INT NOT NULL UNIQUE,
    start_date VARCHAR(10) NOT NULL,
    num_pay_periods INT NOT NULL CHECK(num_pay_periods IN (26, 27)),
    is_active TINYINT NOT NULL DEFAULT 0 CHECK(is_active IN (0, 1)),
    expected_area_count INT
) ENGINE=InnoDB;

CREATE TABLE areas (
    area_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    area_code VARCHAR(255) NOT NULL,
    area_name VARCHAR(255),
    expected_user_count INT,
    UNIQUE (bid_year_id, area_code),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id)
) ENGINE=InnoDB;

CREATE TABLE users (
    user_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    area_id BIGINT NOT NULL,
    initials VARCHAR(10) NOT NULL,
    name VARCHAR(255) NOT NULL,
    user_type VARCHAR(50) NOT NULL,
    crew INT,
    cumulative_natca_bu_date VARCHAR(10) NOT NULL,
    natca_bu_date VARCHAR(10) NOT NULL,
    eod_faa_date VARCHAR(10) NOT NULL,
    service_computation_date VARCHAR(10) NOT NULL,
    lottery_value INT,
    UNIQUE (bid_year_id, area_id, initials),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id)
) ENGINE=InnoDB;

CREATE INDEX idx_users_by_area ON users(bid_year_id, area_id);
CREATE INDEX idx_users_by_initials ON users(bid_year_id, area_id, initials);
