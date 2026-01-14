-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Operator tables (Phase 14)
CREATE TABLE operators (
    operator_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    login_name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('Admin', 'Bidder')),
    is_disabled INTEGER NOT NULL DEFAULT 0 CHECK(is_disabled IN (0, 1)),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disabled_at DATETIME,
    last_login_at DATETIME
);

CREATE TABLE sessions (
    session_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    session_token TEXT NOT NULL UNIQUE,
    operator_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_activity_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    FOREIGN KEY(operator_id) REFERENCES operators(operator_id)
);

CREATE INDEX idx_sessions_token ON sessions(session_token);
CREATE INDEX idx_sessions_operator ON sessions(operator_id);
