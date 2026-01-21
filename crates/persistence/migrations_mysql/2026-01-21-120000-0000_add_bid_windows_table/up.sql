-- Add bid_windows table to store calculated bid windows after confirmation
CREATE TABLE bid_windows (
    bid_window_id BIGINT PRIMARY KEY AUTO_INCREMENT NOT NULL,
    bid_year_id BIGINT NOT NULL,
    area_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    window_start_datetime TEXT NOT NULL,
    window_end_datetime TEXT NOT NULL,
    UNIQUE (bid_year_id, area_id, user_id),
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY (area_id) REFERENCES areas(area_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
) ENGINE=InnoDB;
