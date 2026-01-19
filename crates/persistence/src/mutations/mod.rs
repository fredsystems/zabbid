// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Backend-agnostic mutation modules.
//!
//! This module contains all state-changing operations for the persistence layer.
//! Most mutations use Diesel DSL and are backend-agnostic, with minimal use of
//! backend-specific helpers (e.g., `last_insert_rowid()` for `SQLite`).
//!
//! ## Module Organization
//!
//! - `audit` — Audit event and snapshot persistence
//! - `canonical` — Canonical entity mutations (users, bid years, areas)
//! - `operators` — Operator and session mutations
//! - `bootstrap` — High-level orchestration (`persist_transition`, `persist_bootstrap`)
//!
//! ## Backend-Specific Code
//!
//! Backend-specific helpers (e.g., `get_last_insert_rowid()`) are imported from
//! the `backend` module. All other code uses Diesel DSL exclusively.

pub mod audit;
pub mod bootstrap;
pub mod canonical;
pub mod operators;

// Re-export backend-specific mutation functions used by lib.rs
pub use audit::{persist_audit_event_mysql, persist_audit_event_sqlite};
pub use bootstrap::{
    PersistTransitionResult, persist_bootstrap_mysql, persist_bootstrap_sqlite,
    persist_transition_mysql, persist_transition_sqlite, set_active_bid_year_mysql,
    set_active_bid_year_sqlite, set_expected_area_count_mysql, set_expected_area_count_sqlite,
    set_expected_user_count_mysql, set_expected_user_count_sqlite,
};
pub use canonical::{
    create_system_area_mysql, create_system_area_sqlite, update_area_name_mysql,
    update_area_name_sqlite, update_user_mysql, update_user_sqlite,
};
pub use operators::{
    create_operator_mysql, create_operator_sqlite, create_session_mysql, create_session_sqlite,
    delete_expired_sessions_mysql, delete_expired_sessions_sqlite, delete_operator_mysql,
    delete_operator_sqlite, delete_session_mysql, delete_session_sqlite,
    delete_sessions_for_operator_mysql, delete_sessions_for_operator_sqlite,
    disable_operator_mysql, disable_operator_sqlite, enable_operator_mysql, enable_operator_sqlite,
    update_last_login_mysql, update_last_login_sqlite, update_password_mysql,
    update_password_sqlite, update_session_activity_mysql, update_session_activity_sqlite,
};
