-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

DROP INDEX IF EXISTS idx_users_by_initials;
DROP INDEX IF EXISTS idx_users_by_area;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS areas;
DROP TABLE IF EXISTS bid_years;
