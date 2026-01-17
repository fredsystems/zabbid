-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 25A: Add lifecycle_state to bid_years
ALTER TABLE bid_years ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'Draft';
