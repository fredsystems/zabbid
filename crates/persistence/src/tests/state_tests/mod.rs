// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod audit_timeline_tests;
mod current_state_tests;
mod historical_state_tests;
mod persistence_tests;

// FIXME: We need to have a test to verify time stamps didn't break.
//
// Perfect! Now the admin user should work correctly. The session validation will now succeed, and the role will be properly recognized as "Admin", giving you the correct capabilities.

// ## Summary of Final Fix

// ### The Real Problem
// The session validation was failing because:
// 1. We write timestamps with microseconds: `2026-02-14 16:46:32.123456`
// 2. MySQL's `DATETIME` (without precision specifier) only stores seconds: `2026-02-14 16:46:32`
// 3. The parsing code expected the exact format we wrote, including microseconds
// 4. When MySQL returned the truncated format (no decimal point), parsing failed

// ### The Solution
// Made the timestamp parser flexible to handle **both** formats:
// - **With microseconds**: `YYYY-MM-DD HH:MM:SS.uuuuuu` (when `.` is present)
// - **Without microseconds**: `YYYY-MM-DD HH:MM:SS` (when no `.` is present)

// This works correctly with:
// - **SQLite**: Stores full precision, parser handles it
// - **MySQL DATETIME**: Truncates to seconds, parser handles it
// - **MySQL DATETIME(6)**: Would store full precision if we had it

// ### Why This Fixes Capabilities
// With session validation working:
// 1. Session token validates successfully ✅
// 2. Operator record is retrieved with `role = "Admin"` ✅
// 3. `AuthenticatedActor` is created with `Role::Admin` ✅
// 4. `compute_global_capabilities()` sees Admin role ✅
// 5. Returns Admin capabilities (can create operators, bid years, areas, etc.) ✅

// ### Future Improvement
// For new databases or future migrations, the MySQL schema should use `DATETIME(6)` for microsecond precision. But the current code handles the existing schema gracefully.
