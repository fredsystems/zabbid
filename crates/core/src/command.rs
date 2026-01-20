// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use time::Date;
use zab_bid_domain::{Area, Crew, Initials, SeniorityData, UserType};

/// A command represents user or system intent as data only.
///
/// Commands are the only way to request state changes.
///
/// # Identity and Canonical IDs
///
/// Commands that target specific users MUST carry `user_id` explicitly.
/// This enforces the architectural invariant that canonical identity is
/// explicit and non-negotiable.
///
/// User-targeting commands include both canonical identity (`user_id`) and
/// domain vocabulary (`Initials`, `Area`) to support:
/// - Explicit, authoritative identity (`user_id` for state lookups)
/// - Mutable metadata updates (`initials` may be changed)
/// - Audit trail clarity (denormalized display values)
///
/// The API layer is responsible for:
/// - Resolving display metadata to canonical IDs before constructing commands
/// - Passing `user_id` explicitly into all user-targeting commands
///
/// The core layer MUST:
/// - Use `user_id` for all state lookups and mutations
/// - Never search state by `initials` or other display metadata
/// - Treat `initials` as mutable metadata only, never as identity
///
/// The persistence layer enforces canonical identity:
/// - `user_id` is the sole authoritative identifier for users
/// - `initials` are mutable display metadata, not identifiers
/// - All mutations and foreign keys use canonical IDs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Create a new bid year with canonical metadata.
    CreateBidYear {
        /// The year value.
        year: u16,
        /// The start date of the bid year.
        start_date: Date,
        /// The number of pay periods (must be 26 or 27).
        num_pay_periods: u8,
    },
    /// Create a new area within the active bid year.
    CreateArea {
        /// The area identifier.
        area_id: String,
    },
    /// Register a new user for the active bid year.
    RegisterUser {
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
    /// Set the active bid year (only one can be active at a time).
    SetActiveBidYear {
        /// The year to mark as active.
        year: u16,
    },
    /// Set the expected number of areas for the active bid year.
    SetExpectedAreaCount {
        /// The expected number of areas.
        expected_count: u32,
    },
    /// Set the expected number of users for an area in the active bid year.
    SetExpectedUserCount {
        /// The area.
        area: Area,
        /// The expected number of users.
        expected_count: u32,
    },
    /// Update an existing user's information in the active bid year.
    ///
    /// The user is identified by `user_id` (canonical, immutable).
    /// Initials are mutable metadata and may be changed via this command.
    UpdateUser {
        /// The user's canonical identifier (immutable, authoritative).
        user_id: i64,
        /// The user's initials (mutable metadata, may be updated).
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
    /// Transition a bid year from `Draft` to `BootstrapComplete`.
    TransitionToBootstrapComplete {
        /// The year to transition.
        year: u16,
    },
    /// Transition a bid year from `BootstrapComplete` to `Canonicalized`.
    TransitionToCanonicalized {
        /// The year to transition.
        year: u16,
    },
    /// Transition a bid year from `Canonicalized` to `BiddingActive`.
    TransitionToBiddingActive {
        /// The year to transition.
        year: u16,
    },
    /// Transition a bid year from `BiddingActive` to `BiddingClosed`.
    TransitionToBiddingClosed {
        /// The year to transition.
        year: u16,
    },
    /// Override a user's area assignment after canonicalization.
    ///
    /// The user is identified by `user_id` (canonical, immutable).
    /// Initials are included for audit trail clarity only.
    OverrideAreaAssignment {
        /// The user's canonical identifier (immutable, authoritative).
        user_id: i64,
        /// The user's initials (metadata for audit trail only).
        initials: Initials,
        /// The new area to assign.
        new_area: Area,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
    /// Override a user's eligibility status after canonicalization.
    ///
    /// The user is identified by `user_id` (canonical, immutable).
    /// Initials are included for audit trail clarity only.
    OverrideEligibility {
        /// The user's canonical identifier (immutable, authoritative).
        user_id: i64,
        /// The user's initials (metadata for audit trail only).
        initials: Initials,
        /// The new eligibility status.
        can_bid: bool,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
    /// Override a user's bid order after canonicalization.
    ///
    /// The user is identified by `user_id` (canonical, immutable).
    /// Initials are included for audit trail clarity only.
    OverrideBidOrder {
        /// The user's canonical identifier (immutable, authoritative).
        user_id: i64,
        /// The user's initials (metadata for audit trail only).
        initials: Initials,
        /// The new bid order (or None to clear).
        bid_order: Option<i32>,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
    /// Override a user's bid window after canonicalization.
    ///
    /// The user is identified by `user_id` (canonical, immutable).
    /// Initials are included for audit trail clarity only.
    OverrideBidWindow {
        /// The user's canonical identifier (immutable, authoritative).
        user_id: i64,
        /// The user's initials (metadata for audit trail only).
        initials: Initials,
        /// The new window start date (or None to clear).
        window_start: Option<Date>,
        /// The new window end date (or None to clear).
        window_end: Option<Date>,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
    /// Update a user's participation flags.
    ///
    /// Phase 29A: Controls bid order derivation and leave calculation inclusion.
    /// The user is identified by `user_id` (canonical, immutable).
    /// Initials are included for audit trail clarity only.
    ///
    /// Directional invariant enforced:
    /// `excluded_from_leave_calculation == true` ⇒ `excluded_from_bidding == true`
    UpdateUserParticipation {
        /// The user's canonical identifier (immutable, authoritative).
        user_id: i64,
        /// The user's initials (metadata for audit trail only).
        initials: Initials,
        /// Whether the user is excluded from bidding.
        excluded_from_bidding: bool,
        /// Whether the user is excluded from leave calculation.
        excluded_from_leave_calculation: bool,
    },
}
