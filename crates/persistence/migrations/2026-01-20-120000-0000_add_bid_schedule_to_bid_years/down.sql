-- Remove bid schedule fields from bid_years table

ALTER TABLE bid_years DROP COLUMN bidders_per_area_per_day;
ALTER TABLE bid_years DROP COLUMN bid_window_end_time;
ALTER TABLE bid_years DROP COLUMN bid_window_start_time;
ALTER TABLE bid_years DROP COLUMN bid_start_date;
ALTER TABLE bid_years DROP COLUMN bid_timezone;
