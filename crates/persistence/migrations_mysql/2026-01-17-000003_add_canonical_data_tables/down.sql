-- Drop canonical data tables in reverse order
DROP TABLE IF EXISTS canonical_bid_windows;

DROP TABLE IF EXISTS canonical_bid_order;

DROP TABLE IF EXISTS canonical_eligibility;

DROP INDEX IF EXISTS idx_canonical_area_membership_area ON canonical_area_membership;
DROP TABLE IF EXISTS canonical_area_membership;
