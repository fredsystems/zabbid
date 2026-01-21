-- Add bid_status table to track user bidding progress through rounds
CREATE TABLE bid_status (
    bid_status_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    area_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    round_id BIGINT NOT NULL,
    status VARCHAR(50) NOT NULL,
    updated_at VARCHAR(64) NOT NULL,
    updated_by BIGINT NOT NULL,
    notes TEXT,
    UNIQUE KEY unique_bid_status (bid_year_id, area_id, user_id, round_id),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id),
    FOREIGN KEY(user_id) REFERENCES users(user_id),
    FOREIGN KEY(round_id) REFERENCES rounds(round_id),
    FOREIGN KEY(updated_by) REFERENCES operators(operator_id)
) ENGINE=InnoDB;

-- Add bid_status_history table to track all status transitions
CREATE TABLE bid_status_history (
    history_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_status_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    previous_status VARCHAR(50),
    new_status VARCHAR(50) NOT NULL,
    transitioned_at VARCHAR(64) NOT NULL,
    transitioned_by BIGINT NOT NULL,
    notes TEXT,
    FOREIGN KEY(bid_status_id) REFERENCES bid_status(bid_status_id),
    FOREIGN KEY(audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY(transitioned_by) REFERENCES operators(operator_id)
) ENGINE=InnoDB;

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
