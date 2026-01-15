-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Operator tables (Phase 14) - MySQL/MariaDB version
CREATE TABLE operators (
    operator_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    login_name VARCHAR(255) NOT NULL UNIQUE,
    display_name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL CHECK(role IN ('Admin', 'Bidder')),
    is_disabled TINYINT NOT NULL DEFAULT 0 CHECK(is_disabled IN (0, 1)),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disabled_at DATETIME,
    last_login_at DATETIME
) ENGINE=InnoDB;

CREATE TABLE sessions (
    session_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    session_token VARCHAR(255) NOT NULL UNIQUE,
    operator_id BIGINT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_activity_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    FOREIGN KEY(operator_id) REFERENCES operators(operator_id)
) ENGINE=InnoDB;

CREATE INDEX idx_sessions_token ON sessions(session_token);
CREATE INDEX idx_sessions_operator ON sessions(operator_id);
