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

// Re-export commonly used mutation functions
pub use audit::persist_audit_event;
pub use bootstrap::{
    persist_bootstrap, persist_transition, set_active_bid_year, set_expected_area_count,
    set_expected_user_count,
};
pub use canonical::update_user;
pub use operators::{
    create_operator, create_session, delete_expired_sessions, delete_operator, delete_session,
    delete_sessions_for_operator, disable_operator, enable_operator, update_last_login,
    update_password, update_session_activity,
};
