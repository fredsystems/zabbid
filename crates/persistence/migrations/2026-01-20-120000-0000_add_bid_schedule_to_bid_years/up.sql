-- Add bid schedule fields to bid_years table
-- All fields are nullable until confirmation
-- At confirmation time, all fields must be non-null

ALTER TABLE bid_years ADD COLUMN bid_timezone TEXT;
ALTER TABLE bid_years ADD COLUMN bid_start_date TEXT;
ALTER TABLE bid_years ADD COLUMN bid_window_start_time TEXT;
ALTER TABLE bid_years ADD COLUMN bid_window_end_time TEXT;
ALTER TABLE bid_years ADD COLUMN bidders_per_area_per_day INTEGER;
