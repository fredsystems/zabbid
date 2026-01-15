// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Query modules for persistence layer.
//!
//! This module contains all read-only queries for the persistence layer.
//!
//! ## Module Organization
//!
//! - `audit` — Audit event queries
//! - `state` — State snapshot and reconstruction queries
//! - `canonical` — Canonical entity queries (bid years, areas, users)
//! - `operators` — Operator and session queries
//! - `completeness` — Count and aggregation queries
//!
//! ## Backend-Specific Functions
//!
//! All query functions are generated in backend-specific monomorphic versions:
//! - Functions suffixed with `_sqlite` for `SQLite`
//! - Functions suffixed with `_mysql` for `MySQL`/`MariaDB`
//!
//! The `Persistence` adapter in `lib.rs` dispatches to the appropriate version
//! based on the active backend connection.

pub mod audit;
pub mod canonical;
pub mod completeness;
pub mod operators;
pub mod state;

// Re-export the should_snapshot helper (not backend-specific)
pub use state::should_snapshot;

// Re-export backend-specific query functions used by lib.rs
pub use audit::{
    get_audit_timeline_mysql, get_audit_timeline_sqlite, get_events_after_mysql,
    get_events_after_sqlite, get_global_audit_events_mysql, get_global_audit_events_sqlite,
};
pub use canonical::{
    get_bootstrap_metadata_mysql, get_bootstrap_metadata_sqlite, list_areas_mysql,
    list_areas_sqlite, list_bid_years_mysql, list_bid_years_sqlite, list_users_mysql,
    list_users_sqlite, lookup_area_id_mysql, lookup_area_id_sqlite, lookup_bid_year_id_mysql,
    lookup_bid_year_id_sqlite,
};
pub use completeness::{
    count_areas_by_bid_year_mysql, count_areas_by_bid_year_sqlite, count_users_by_area_mysql,
    count_users_by_area_sqlite, count_users_by_bid_year_and_area_mysql,
    count_users_by_bid_year_and_area_sqlite, count_users_by_bid_year_mysql,
    count_users_by_bid_year_sqlite,
};
pub use state::{
    get_current_state_mysql, get_current_state_sqlite, get_historical_state_mysql,
    get_historical_state_sqlite, get_latest_snapshot_mysql, get_latest_snapshot_sqlite,
};
