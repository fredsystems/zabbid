// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Database backend-specific code.
//!
//! This module isolates backend-specific initialization, migration,
//! and helper functions that cannot be expressed in backend-agnostic
//! Diesel DSL.
//!
//! ## Backend Support
//!
//! - `sqlite` — `SQLite` backend (default for development and testing)
//! - `mysql` — MySQL/MariaDB backend (validated via opt-in tests)
//!
//! ## Backend-Agnostic Code
//!
//! Most persistence code should be backend-agnostic and use Diesel DSL.
//! Backend-specific code is limited to:
//!
//! - Connection initialization
//! - Migration execution
//! - Backend-specific configuration (e.g., PRAGMA, engine settings)
//! - Backend-specific workarounds for missing Diesel DSL features
//!
//! All domain queries and mutations live in `queries/` and `mutations/`
//! modules and must work across all supported backends.

pub mod mysql;
pub mod sqlite;
