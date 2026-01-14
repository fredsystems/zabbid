-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Audit log and derived historical state tables
-- Phase 23A: Now use canonical IDs with FKs, but area_id can be NULL for CreateBidYear
-- Phase 23B: bid_year_id can also be NULL for global events (operator management)
CREATE TABLE audit_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER,
    area_id INTEGER,
    year INTEGER NOT NULL,
    area_code TEXT NOT NULL,
    actor_operator_id INTEGER NOT NULL,
    actor_login_name TEXT NOT NULL,
    actor_display_name TEXT NOT NULL,
    actor_json TEXT NOT NULL,
    cause_json TEXT NOT NULL,
    action_json TEXT NOT NULL,
    before_snapshot_json TEXT NOT NULL,
    after_snapshot_json TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(actor_operator_id) REFERENCES operators(operator_id) ON DELETE RESTRICT,
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id)
);

CREATE INDEX idx_audit_events_scope ON audit_events(bid_year_id, area_id, event_id);

CREATE TABLE state_snapshots (
    snapshot_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    event_id INTEGER NOT NULL,
    state_json TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bid_year_id, area_id, event_id),
    FOREIGN KEY(event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id)
);

CREATE INDEX idx_state_snapshots_scope ON state_snapshots(bid_year_id, area_id, event_id DESC);
