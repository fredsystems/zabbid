// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Backend-agnostic query modules.
//!
//! This module contains all read-only queries for the persistence layer.
//! All queries use Diesel DSL and are backend-agnostic, working across
//! `SQLite`, MySQL/MariaDB, and any other supported database backend.
//!
//! ## Module Organization
//!
//! - `audit` — Audit event queries
//! - `state` — State snapshot and reconstruction queries
//! - `canonical` — Canonical entity queries (bid years, areas, users)
//! - `operators` — Operator and session queries
//! - `completeness` — Count and aggregation queries
//!
//! ## Backend-Agnostic Queries
//!
//! All functions in this module use Diesel DSL exclusively.
//! Backend-specific code (e.g., `last_insert_rowid()`) lives in the `backend` module.

pub mod audit;
pub mod canonical;
pub mod completeness;
pub mod operators;
pub mod state;

// Re-export commonly used query functions
pub use audit::{get_audit_event, get_audit_timeline, get_events_after, get_global_audit_events};
pub use canonical::{
    get_active_bid_year, get_actual_area_count, get_actual_user_count, get_bootstrap_metadata,
    get_expected_area_count, get_expected_user_count, list_areas, list_bid_years, list_users,
    lookup_area_id, lookup_bid_year_id,
};
pub use completeness::{
    count_areas_by_bid_year, count_users_by_area, count_users_by_bid_year,
    count_users_by_bid_year_and_area,
};
pub use operators::{
    count_active_admin_operators, count_operators, get_operator_by_id, get_operator_by_login,
    get_session_by_token, is_operator_referenced, list_operators, verify_password,
};
pub use state::{get_current_state, get_historical_state, get_latest_snapshot, should_snapshot};
