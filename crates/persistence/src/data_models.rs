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
pub type AuditEventRow = (i64, u16, String, String, String, String, String, String);
