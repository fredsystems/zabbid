-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

DROP INDEX IF EXISTS idx_sessions_operator;
DROP INDEX IF EXISTS idx_sessions_token;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS operators;
