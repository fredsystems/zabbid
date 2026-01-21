-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 29A: Add user participation flags
-- These flags control bid order derivation, readiness evaluation, and round capacity calculations.
-- They do NOT trigger execution behavior or time-based transitions.

-- Add excluded_from_bidding flag (default: false/0)
-- If true, user is excluded from bid order derivation and does not receive a bid window.
ALTER TABLE users ADD COLUMN excluded_from_bidding TINYINT NOT NULL DEFAULT 0 CHECK(excluded_from_bidding IN (0, 1));

-- Add excluded_from_leave_calculation flag (default: false/0)
-- If true, user does not count toward area leave capacity or maximum bid slots.
ALTER TABLE users ADD COLUMN excluded_from_leave_calculation TINYINT NOT NULL DEFAULT 0 CHECK(excluded_from_leave_calculation IN (0, 1));

-- Directional Invariant (enforced at application layer, documented here):
-- excluded_from_leave_calculation == true ⇒ excluded_from_bidding == true
-- A user may never be included in bidding while excluded from leave calculation.
