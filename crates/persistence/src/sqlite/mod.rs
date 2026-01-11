// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod bootstrap;
mod operators;
mod persistence;
mod queries;
mod schema;

pub use bootstrap::{
    get_bootstrap_metadata, list_areas, list_bid_years, list_users, verify_foreign_key_enforcement,
};
pub use operators::{
    create_operator, create_session, delete_expired_sessions, delete_session, disable_operator,
    get_operator_by_id, get_operator_by_login, get_session_by_token, is_operator_referenced,
    list_operators, update_last_login, update_session_activity,
};
pub use persistence::{persist_bootstrap, persist_transition};
pub use queries::{
    count_areas_by_bid_year, count_users_by_area, count_users_by_bid_year,
    count_users_by_bid_year_and_area, get_audit_event, get_audit_timeline, get_current_state,
    get_events_after, get_historical_state, get_latest_snapshot, should_snapshot,
};
pub use schema::initialize_schema;
