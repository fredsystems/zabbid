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
pub mod bid_status;
pub mod canonical;
pub mod completeness;
pub mod operators;
pub mod readiness;
pub mod rounds;
pub mod state;

// Re-export the should_snapshot helper (not backend-specific)
pub use state::should_snapshot;

// Re-export backend-specific query functions used by lib.rs
pub use audit::{
    get_audit_timeline_mysql, get_audit_timeline_sqlite, get_events_after_mysql,
    get_events_after_sqlite, get_global_audit_events_mysql, get_global_audit_events_sqlite,
};
pub use canonical::{
    count_users_in_system_area_mysql, count_users_in_system_area_sqlite, find_system_area_mysql,
    find_system_area_sqlite, get_bootstrap_metadata_mysql, get_bootstrap_metadata_sqlite,
    is_system_area_mysql, is_system_area_sqlite, list_areas_mysql, list_areas_sqlite,
    list_bid_years_mysql, list_bid_years_sqlite, list_users_in_system_area_mysql,
    list_users_in_system_area_sqlite, list_users_mysql, list_users_sqlite, lookup_area_id_mysql,
    lookup_area_id_sqlite, lookup_bid_year_id_mysql, lookup_bid_year_id_sqlite,
};
pub use completeness::{
    count_areas_by_bid_year_mysql, count_areas_by_bid_year_sqlite, count_users_by_area_mysql,
    count_users_by_area_sqlite, count_users_by_bid_year_and_area_mysql,
    count_users_by_bid_year_and_area_sqlite, count_users_by_bid_year_mysql,
    count_users_by_bid_year_sqlite,
};
// Phase 29D: Readiness query re-exports
// These are used indirectly via Persistence wrapper methods in lib.rs
#[allow(unused_imports)]
pub use readiness::{
    count_participation_flag_violations_mysql, count_participation_flag_violations_sqlite,
    count_unreviewed_no_bid_users_mysql, count_unreviewed_no_bid_users_sqlite,
    get_areas_missing_rounds_mysql, get_areas_missing_rounds_sqlite,
    get_users_by_area_for_conflict_detection_mysql,
    get_users_by_area_for_conflict_detection_sqlite, is_bid_schedule_set_mysql,
    is_bid_schedule_set_sqlite, mark_user_no_bid_reviewed_mysql, mark_user_no_bid_reviewed_sqlite,
};
#[allow(unused_imports)]
pub use rounds::{
    count_rounds_using_group_mysql, count_rounds_using_group_sqlite, delete_round_group_mysql,
    delete_round_group_sqlite, delete_round_mysql, delete_round_sqlite, get_round_group_mysql,
    get_round_group_sqlite, get_round_mysql, get_round_sqlite, insert_round_group_mysql,
    insert_round_group_sqlite, insert_round_mysql, insert_round_sqlite, list_round_groups_mysql,
    list_round_groups_sqlite, list_rounds_mysql, list_rounds_sqlite, round_group_name_exists_mysql,
    round_group_name_exists_sqlite, round_number_exists_mysql, round_number_exists_sqlite,
    update_round_group_mysql, update_round_group_sqlite, update_round_mysql, update_round_sqlite,
};
pub use state::{
    get_current_state_mysql, get_current_state_sqlite, get_historical_state_mysql,
    get_historical_state_sqlite, get_latest_snapshot_mysql, get_latest_snapshot_sqlite,
};

// Phase 29F: Bid status query re-exports
#[allow(unused_imports)]
pub use bid_status::{
    get_bid_status_for_area_mysql, get_bid_status_for_area_sqlite, get_bid_status_for_round_mysql,
    get_bid_status_for_round_sqlite, get_bid_status_for_user_and_round_mysql,
    get_bid_status_for_user_and_round_sqlite, get_bid_status_history_mysql,
    get_bid_status_history_sqlite,
};
