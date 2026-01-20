-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 29A: Revert user participation flags

-- MySQL/MariaDB supports DROP COLUMN directly
ALTER TABLE users DROP COLUMN excluded_from_leave_calculation;
ALTER TABLE users DROP COLUMN excluded_from_bidding;
