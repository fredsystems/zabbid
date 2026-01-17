-- Canonical Area Membership
-- Captures which users are assigned to which areas at canonicalization
CREATE TABLE canonical_area_membership (
    id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    area_id BIGINT NOT NULL,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    INDEX idx_canonical_area_membership_area (bid_year_id, area_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (area_id) REFERENCES areas(area_id)
) ENGINE=InnoDB;

-- Canonical Eligibility
-- Captures whether a user is eligible to bid
CREATE TABLE canonical_eligibility (
    id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    can_bid TINYINT NOT NULL,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
) ENGINE=InnoDB;

-- Canonical Bid Order
-- Captures bid order (NULL until computed)
CREATE TABLE canonical_bid_order (
    id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    bid_order INT,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
) ENGINE=InnoDB;

-- Canonical Bid Windows
-- Captures bid submission windows (NULL until computed)
CREATE TABLE canonical_bid_windows (
    id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    window_start_date VARCHAR(50),
    window_end_date VARCHAR(50),
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    UNIQUE (bid_year_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(event_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
) ENGINE=InnoDB;
