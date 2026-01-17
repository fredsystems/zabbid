-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 25B: Add system area flag to areas table
ALTER TABLE areas ADD COLUMN is_system_area INTEGER NOT NULL DEFAULT 0 CHECK(is_system_area IN (0, 1));
