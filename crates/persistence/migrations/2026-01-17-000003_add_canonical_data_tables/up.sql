-- Canonical Area Membership
-- Captures which users are assigned to which areas at canonicalization
CREATE TABLE canonical_area_membership (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (area_id) REFERENCES areas(area_id)
);

CREATE INDEX idx_canonical_area_membership_area
    ON canonical_area_membership(bid_year_id, area_id);

-- Canonical Eligibility
-- Captures whether a user is eligible to bid
CREATE TABLE canonical_eligibility (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    can_bid INTEGER NOT NULL,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

-- Canonical Bid Order
-- Captures bid order (NULL until computed)
CREATE TABLE canonical_bid_order (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    bid_order INTEGER,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

-- Canonical Bid Windows
-- Captures bid submission windows (NULL until computed)
CREATE TABLE canonical_bid_windows (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    window_start_date TEXT,
    window_end_date TEXT,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);
