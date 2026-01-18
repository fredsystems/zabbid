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
/// Commands use domain vocabulary (e.g., `Initials`, `Area`) rather than
/// persistence identifiers (e.g., `user_id`, `area_id`). This keeps the core
/// layer UI-agnostic and focused on domain semantics.
///
/// The API layer is responsible for translating between:
/// - External canonical identifiers (`user_id`, `area_id`, `bid_year_id`)
/// - Domain vocabulary used in commands (`Initials`, `Area`, `BidYear`)
///
/// The persistence layer enforces canonical identity:
/// - `user_id` is the sole authoritative identifier for users
/// - `initials` are mutable display metadata, not identifiers
/// - All mutations and foreign keys use canonical IDs
///
/// This architectural separation allows:
/// - Domain rules to be expressed in business terms
/// - Display fields (like initials) to change without breaking references
/// - Persistence layer to enforce referential integrity via canonical IDs
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
    /// Note: The API layer identifies users by `user_id` (canonical identifier).
    /// This command uses `initials` as domain vocabulary for the update operation.
    /// Initials are mutable and may be changed via this command.
    UpdateUser {
        /// The user's initials (may be updated).
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
    /// Note: The API layer resolves `user_id` before constructing this command.
    /// Initials are used here as domain vocabulary, not as the persistence key.
    OverrideAreaAssignment {
        /// The user's initials (domain reference, resolved via `user_id` at API boundary).
        initials: Initials,
        /// The new area to assign.
        new_area: Area,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
    /// Override a user's eligibility status after canonicalization.
    ///
    /// Note: The API layer resolves `user_id` before constructing this command.
    /// Initials are used here as domain vocabulary, not as the persistence key.
    OverrideEligibility {
        /// The user's initials (domain reference, resolved via `user_id` at API boundary).
        initials: Initials,
        /// The new eligibility status.
        can_bid: bool,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
    /// Override a user's bid order after canonicalization.
    ///
    /// Note: The API layer resolves `user_id` before constructing this command.
    /// Initials are used here as domain vocabulary, not as the persistence key.
    OverrideBidOrder {
        /// The user's initials (domain reference, resolved via `user_id` at API boundary).
        initials: Initials,
        /// The new bid order (or None to clear).
        bid_order: Option<i32>,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
    /// Override a user's bid window after canonicalization.
    ///
    /// Note: The API layer resolves `user_id` before constructing this command.
    /// Initials are used here as domain vocabulary, not as the persistence key.
    OverrideBidWindow {
        /// The user's initials (domain reference, resolved via `user_id` at API boundary).
        initials: Initials,
        /// The new window start date (or None to clear).
        window_start: Option<Date>,
        /// The new window end date (or None to clear).
        window_end: Option<Date>,
        /// The reason for the override (must be non-empty, min 10 chars).
        reason: String,
    },
}
