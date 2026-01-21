-- Add bid_status table to track user bidding progress through rounds
CREATE TABLE bid_status (
    bid_status_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    round_id INTEGER NOT NULL,
    status TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    updated_by INTEGER NOT NULL,
    notes TEXT,
    UNIQUE (bid_year_id, area_id, user_id, round_id),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id),
    FOREIGN KEY(user_id) REFERENCES users(user_id),
    FOREIGN KEY(round_id) REFERENCES rounds(round_id),
    FOREIGN KEY(updated_by) REFERENCES operators(operator_id)
);

-- Add bid_status_history table to track all status transitions
CREATE TABLE bid_status_history (
    history_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_status_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    previous_status TEXT,
    new_status TEXT NOT NULL,
    transitioned_at TEXT NOT NULL,
    transitioned_by INTEGER NOT NULL,
    notes TEXT,
    FOREIGN KEY(bid_status_id) REFERENCES bid_status(bid_status_id),
    FOREIGN KEY(audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY(transitioned_by) REFERENCES operators(operator_id)
);

-- Index for querying status by bid year and area
CREATE INDEX idx_bid_status_bid_year_area ON bid_status(bid_year_id, area_id);

-- Index for querying status by user
CREATE INDEX idx_bid_status_user ON bid_status(user_id);

-- Index for querying status by round
CREATE INDEX idx_bid_status_round ON bid_status(round_id);

-- Index for querying history by bid_status_id
CREATE INDEX idx_bid_status_history_bid_status ON bid_status_history(bid_status_id);

-- Index for querying history by audit event
CREATE INDEX idx_bid_status_history_audit_event ON bid_status_history(audit_event_id);
