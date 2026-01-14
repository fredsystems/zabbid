-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

DROP INDEX IF EXISTS idx_state_snapshots_scope;
DROP TABLE IF EXISTS state_snapshots;
DROP INDEX IF EXISTS idx_audit_events_scope;
DROP TABLE IF EXISTS audit_events;
