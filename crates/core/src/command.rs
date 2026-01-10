// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use zab_bid_domain::{Area, BidYear, Crew, Initials, SeniorityData, UserType};

/// A command represents user or system intent as data only.
///
/// Commands are the only way to request state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Create a new bid year.
    CreateBidYear {
        /// The year value.
        year: u16,
    },
    /// Create a new area within a bid year.
    CreateArea {
        /// The bid year this area belongs to.
        bid_year: BidYear,
        /// The area identifier.
        area_id: String,
    },
    /// Register a new user for a bid year.
    RegisterUser {
        /// The bid year.
        bid_year: BidYear,
        /// The user's initials.
        initials: Initials,
        /// The user's name.
        name: String,
        /// The user's area.
        area: Area,
        /// The user's type classification.
        user_type: UserType,
        /// The user's crew (optional).
        crew: Option<Crew>,
        /// The user's seniority data.
        seniority_data: SeniorityData,
    },
    /// Create an explicit checkpoint, triggering a full state snapshot.
    Checkpoint,
    /// Mark a milestone as finalized, triggering a full state snapshot.
    Finalize,
    /// Rollback to a specific event ID, establishing it as authoritative going forward.
    /// This creates a new audit event and triggers a full state snapshot.
    RollbackToEventId {
        /// The event ID to rollback to.
        /// Must be within the same `(bid_year, area)` scope.
        target_event_id: i64,
    },
}
