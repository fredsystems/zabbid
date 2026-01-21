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

/// Canonical area membership row (diesel queryable).
#[allow(dead_code)]
#[derive(Debug, Clone, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = crate::diesel_schema::canonical_area_membership)]
pub struct CanonicalAreaMembershipRow {
    pub id: Option<i64>,
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub area_id: i64,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Canonical area membership insertable (diesel insertable).
#[derive(Debug, Clone, diesel::Insertable)]
#[diesel(table_name = crate::diesel_schema::canonical_area_membership)]
pub struct NewCanonicalAreaMembership {
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub area_id: i64,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Canonical eligibility row (diesel queryable).
#[allow(dead_code)]
#[derive(Debug, Clone, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = crate::diesel_schema::canonical_eligibility)]
pub struct CanonicalEligibilityRow {
    pub id: Option<i64>,
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub can_bid: i32,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Canonical eligibility insertable (diesel insertable).
#[derive(Debug, Clone, diesel::Insertable)]
#[diesel(table_name = crate::diesel_schema::canonical_eligibility)]
pub struct NewCanonicalEligibility {
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub can_bid: i32,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Canonical bid order row (diesel queryable).
#[allow(dead_code)]
#[derive(Debug, Clone, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = crate::diesel_schema::canonical_bid_order)]
pub struct CanonicalBidOrderRow {
    pub id: Option<i64>,
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub bid_order: Option<i32>,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Canonical bid order insertable (diesel insertable).
#[derive(Debug, Clone, diesel::Insertable)]
#[diesel(table_name = crate::diesel_schema::canonical_bid_order)]
pub struct NewCanonicalBidOrder {
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub bid_order: Option<i32>,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Bid window row (diesel queryable).
#[allow(dead_code)]
#[derive(Debug, Clone, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = crate::diesel_schema::bid_windows)]
pub struct BidWindowRow {
    pub bid_window_id: Option<i64>,
    pub bid_year_id: i64,
    pub area_id: i64,
    pub user_id: i64,
    pub window_start_datetime: String,
    pub window_end_datetime: String,
}

/// Bid window insertable (diesel insertable).
#[derive(Debug, Clone, diesel::Insertable)]
#[diesel(table_name = crate::diesel_schema::bid_windows)]
pub struct NewBidWindow {
    pub bid_year_id: i64,
    pub area_id: i64,
    pub user_id: i64,
    pub window_start_datetime: String,
    pub window_end_datetime: String,
}

/// Canonical bid windows row (diesel queryable).
#[allow(dead_code)]
#[derive(Debug, Clone, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = crate::diesel_schema::canonical_bid_windows)]
pub struct CanonicalBidWindowsRow {
    pub id: Option<i64>,
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub window_start_date: Option<String>,
    pub window_end_date: Option<String>,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Canonical bid windows insertable (diesel insertable).
#[derive(Debug, Clone, diesel::Insertable)]
#[diesel(table_name = crate::diesel_schema::canonical_bid_windows)]
pub struct NewCanonicalBidWindows {
    pub bid_year_id: i64,
    pub audit_event_id: i64,
    pub user_id: i64,
    pub window_start_date: Option<String>,
    pub window_end_date: Option<String>,
    pub is_overridden: i32,
    pub override_reason: Option<String>,
}

/// Canonicalization snapshot: per-user data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalizedUserSnapshot {
    pub user_id: i64,
    pub initials: String,
    pub name: String,
    pub area_id: i64,
    pub area_code: String,
    pub area_name: String,
    pub can_bid: bool,
    pub bid_order: Option<i32>,
    pub window_start_date: Option<String>,
    pub window_end_date: Option<String>,
}

/// Canonicalization snapshot: per-area data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalizedAreaSnapshot {
    pub area_id: i64,
    pub area_code: String,
    pub area_name: String,
    pub user_count: usize,
}

/// Complete canonicalization snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalizationSnapshot {
    pub bid_year_id: i64,
    pub year: u16,
    pub user_count: usize,
    pub area_count: usize,
    pub users: Vec<CanonicalizedUserSnapshot>,
    pub areas: Vec<CanonicalizedAreaSnapshot>,
    pub timestamp: String,
}

/// Bid status row (diesel queryable).
#[allow(dead_code)]
#[derive(Debug, Clone, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = crate::diesel_schema::bid_status)]
pub struct BidStatusRow {
    pub bid_status_id: i64,
    pub bid_year_id: i64,
    pub area_id: i64,
    pub user_id: i64,
    pub round_id: i64,
    pub status: String,
    pub updated_at: String,
    pub updated_by: i64,
    pub notes: Option<String>,
}

/// Bid status insertable (diesel insertable).
#[derive(Debug, Clone, diesel::Insertable)]
#[diesel(table_name = crate::diesel_schema::bid_status)]
pub struct NewBidStatus {
    pub bid_year_id: i64,
    pub area_id: i64,
    pub user_id: i64,
    pub round_id: i64,
    pub status: String,
    pub updated_at: String,
    pub updated_by: i64,
    pub notes: Option<String>,
}

/// Bid status history row (diesel queryable).
#[allow(dead_code)]
#[derive(Debug, Clone, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = crate::diesel_schema::bid_status_history)]
pub struct BidStatusHistoryRow {
    pub history_id: i64,
    pub bid_status_id: i64,
    pub audit_event_id: i64,
    pub previous_status: Option<String>,
    pub new_status: String,
    pub transitioned_at: String,
    pub transitioned_by: i64,
    pub notes: Option<String>,
}

/// Bid status history insertable (diesel insertable).
#[derive(Debug, Clone, diesel::Insertable)]
#[diesel(table_name = crate::diesel_schema::bid_status_history)]
pub struct NewBidStatusHistory {
    pub bid_status_id: i64,
    pub audit_event_id: i64,
    pub previous_status: Option<String>,
    pub new_status: String,
    pub transitioned_at: String,
    pub transitioned_by: i64,
    pub notes: Option<String>,
}
