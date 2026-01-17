-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 25A: Remove lifecycle_state from bid_years
ALTER TABLE bid_years DROP COLUMN lifecycle_state;
