// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use serde::{Deserialize, Serialize};

/// Serializable representation of an Actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorData {
    pub id: String,
    pub actor_type: String,
}

/// Serializable representation of a Cause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CauseData {
    pub id: String,
    pub description: String,
}

/// Serializable representation of an Action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionData {
    pub name: String,
    pub details: Option<String>,
}

/// Serializable representation of a `StateSnapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshotData {
    pub data: String,
}

/// Serializable representation of the full State.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateData {
    pub bid_year: u16,
    pub area: String,
    pub users_json: String,
}

/// Type alias for audit event row data from `SQLite`.
///
/// Phase 23A: Now includes `bid_year_id` and `area_id` in addition to display values.
///
/// Contains: (`event_id`, `bid_year_id`, `area_id`, `year`, `area_code`,
/// `actor_operator_id`, `actor_login_name`, `actor_display_name`,
/// `actor_json`, `cause_json`, `action_json`, `before_json`, `after_json`)
#[allow(dead_code)]
pub type AuditEventRow = (
    i64,         // event_id
    i64,         // bid_year_id
    Option<i64>, // area_id (nullable for CreateBidYear events - Phase 23A)
    u16,         // year
    String,      // area_code
    i64,         // actor_operator_id
    String,      // actor_login_name
    String,      // actor_display_name
    String,      // actor_json
    String,      // cause_json
    String,      // action_json
    String,      // before_json
    String,      // after_json
);

/// Serializable representation of an Operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorData {
    pub operator_id: i64,
    pub login_name: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: String,
    pub is_disabled: bool,
    pub created_at: String,
    pub disabled_at: Option<String>,
    pub last_login_at: Option<String>,
}

/// Serializable representation of a Session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: i64,
    pub session_token: String,
    pub operator_id: i64,
    pub created_at: String,
    pub last_activity_at: String,
    pub expires_at: String,
}
