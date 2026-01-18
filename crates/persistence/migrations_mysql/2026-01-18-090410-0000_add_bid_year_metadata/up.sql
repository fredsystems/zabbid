-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 26E: Add metadata fields to bid_years
ALTER TABLE bid_years ADD COLUMN label TEXT NULL;
ALTER TABLE bid_years ADD COLUMN notes TEXT NULL;
