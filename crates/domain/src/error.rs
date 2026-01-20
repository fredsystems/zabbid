// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::types::{BidYear, Initials};

/// Errors that can occur during domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// User initials are not unique within the bid year.
    DuplicateInitials {
        /// The bid year in which the duplicate was found.
        bid_year: BidYear,
        /// The duplicate initials.
        initials: Initials,
    },
    /// User initials are empty or invalid.
    InvalidInitials(String),
    /// User name is empty or invalid.
    InvalidName(String),
    /// Area identifier is empty or invalid.
    InvalidArea(String),
    /// Crew identifier is empty or invalid.
    InvalidCrew(&'static str),
    /// User type is invalid.
    InvalidUserType(String),
    /// Bid year does not exist.
    BidYearNotFound(u16),
    /// Area does not exist in the specified bid year.
    AreaNotFound {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// Bid year already exists.
    DuplicateBidYear(u16),
    /// Area already exists in the bid year.
    DuplicateArea {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// Invalid bid year value.
    InvalidBidYear(String),
    /// Invalid pay period count.
    InvalidPayPeriodCount {
        /// The invalid count value.
        count: u8,
    },
    /// Invalid pay period index.
    InvalidPayPeriodIndex {
        /// The invalid index.
        index: u8,
        /// The maximum valid index.
        max: u8,
    },
    /// Date arithmetic overflow.
    DateArithmeticOverflow {
        /// Description of the operation that failed.
        operation: String,
    },
    /// Bid year start date must be a Sunday.
    InvalidStartDateWeekday {
        /// The invalid start date.
        start_date: time::Date,
        /// The actual weekday.
        weekday: time::Weekday,
    },
    /// Bid year start date must be in January.
    InvalidStartDateMonth {
        /// The invalid start date.
        start_date: time::Date,
        /// The actual month.
        month: time::Month,
    },
    /// Service computation date is missing or invalid.
    InvalidServiceComputationDate {
        /// Description of the validation error.
        reason: String,
    },
    /// Failed to parse date from string.
    DateParseError {
        /// The invalid date string.
        date_string: String,
        /// The parsing error message.
        error: String,
    },
    /// User not found.
    UserNotFound {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
        /// The user's initials.
        initials: String,
    },
    /// Multiple bid years cannot be active simultaneously.
    MultipleBidYearsActive {
        /// The currently active year.
        current_active: u16,
        /// The year attempting to become active.
        requested_active: u16,
    },
    /// No active bid year is set.
    NoActiveBidYear,
    /// Expected area count must be positive.
    InvalidExpectedAreaCount {
        /// The invalid count value.
        count: u32,
    },
    /// Expected user count must be positive.
    InvalidExpectedUserCount {
        /// The invalid count value.
        count: u32,
    },
    /// Cannot remove the last active admin operator.
    CannotRemoveLastActiveAdmin,
    /// Invalid lifecycle state string.
    InvalidLifecycleState(String),
    /// Invalid state transition attempted.
    InvalidStateTransition {
        /// The current state.
        current: String,
        /// The requested target state.
        target: String,
    },
    /// Bootstrap must be complete before transitioning to `BootstrapComplete` state.
    BootstrapIncomplete,
    /// Another bid year is already active.
    AnotherBidYearAlreadyActive {
        /// The currently active bid year.
        active_year: u16,
    },
    /// Operation not allowed in current lifecycle state.
    OperationNotAllowedInState {
        /// The operation that was attempted.
        operation: String,
        /// The current lifecycle state.
        state: String,
    },
    /// System area already exists for this bid year.
    SystemAreaAlreadyExists {
        /// The bid year.
        bid_year: u16,
    },
    /// Cannot complete bootstrap while users remain in No Bid area.
    UsersInNoBidArea {
        /// The bid year.
        bid_year: u16,
        /// Count of users still in No Bid area.
        user_count: usize,
        /// Sample of user initials (first 5).
        sample_initials: Vec<String>,
    },
    /// Cannot delete a system area.
    CannotDeleteSystemArea {
        /// The area code.
        area_code: String,
    },
    /// Cannot rename a system area.
    CannotRenameSystemArea {
        /// The area code.
        area_code: String,
    },
    /// Cannot edit area metadata after canonicalization.
    CannotEditAreaAfterCanonicalization {
        /// The bid year.
        bid_year: u16,
        /// The lifecycle state.
        lifecycle_state: String,
    },
    /// Cannot delete users after canonicalization.
    CannotDeleteUserAfterCanonicalization {
        /// The bid year.
        bid_year: u16,
        /// The lifecycle state.
        lifecycle_state: String,
    },
    /// Cannot assign users to No Bid area after canonicalization.
    CannotAssignToNoBidAfterCanonicalization {
        /// The bid year.
        bid_year: u16,
        /// The lifecycle state.
        lifecycle_state: String,
    },
    /// Cannot perform override before canonicalization.
    CannotOverrideBeforeCanonicalization {
        /// The current lifecycle state.
        current_state: String,
    },
    /// Override reason is invalid (empty or too short).
    InvalidOverrideReason {
        /// The reason provided.
        reason: String,
    },
    /// Canonical record not found for override operation.
    CanonicalRecordNotFound {
        /// Description of which record was not found.
        description: String,
    },
    /// Cannot assign user to system area via override.
    CannotAssignToSystemArea {
        /// The system area code.
        area_code: String,
    },
    /// Invalid bid order value.
    InvalidBidOrder {
        /// Description of the validation error.
        reason: String,
    },
    /// Invalid bid window dates.
    InvalidBidWindow {
        /// Description of the validation error.
        reason: String,
    },
    /// Participation flag directional invariant violation.
    /// Phase 29A: `excluded_from_leave_calculation` => `excluded_from_bidding`
    ParticipationFlagViolation {
        /// The user's initials (for error context).
        user_initials: String,
        /// Description of the violation.
        reason: String,
    },
    /// Round group not found.
    /// Phase 29B
    RoundGroupNotFound {
        /// The round group ID.
        round_group_id: i64,
    },
    /// Round group name already exists in the bid year.
    /// Phase 29B
    DuplicateRoundGroupName {
        /// The bid year.
        bid_year: u16,
        /// The round group name.
        name: String,
    },
    /// Round not found.
    /// Phase 29B
    RoundNotFound {
        /// The round ID.
        round_id: i64,
    },
    /// Round number already exists in the area.
    /// Phase 29B
    DuplicateRoundNumber {
        /// The area code.
        area_code: String,
        /// The round number.
        round_number: u32,
    },
    /// Cannot create round for system area.
    /// Phase 29B
    CannotCreateRoundForSystemArea {
        /// The system area code.
        area_code: String,
    },
    /// Invalid round configuration.
    /// Phase 29B
    InvalidRoundConfiguration {
        /// Description of the validation error.
        reason: String,
    },
    /// Cannot delete round group because it is referenced by rounds.
    /// Phase 29B
    RoundGroupInUse {
        /// The round group ID.
        round_group_id: i64,
        /// Number of rounds referencing this group.
        round_count: usize,
    },
    /// Invalid timezone identifier.
    /// Phase 29C
    InvalidTimezone(String),
    /// Bid start date is not a Monday.
    /// Phase 29C
    BidStartDateNotMonday(time::Date),
    /// Bid start date is not in the future.
    /// Phase 29C
    BidStartDateNotFuture {
        /// The bid start date.
        start_date: time::Date,
        /// The reference date (typically "today").
        reference_date: time::Date,
    },
    /// Invalid bid window times.
    /// Phase 29C
    InvalidBidWindowTimes {
        /// Window start time.
        start: time::Time,
        /// Window end time.
        end: time::Time,
    },
    /// Invalid bidders per day count.
    /// Phase 29C
    InvalidBiddersPerDay(u32),
}

impl std::fmt::Display for DomainError {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateInitials { bid_year, initials } => {
                write!(
                    f,
                    "User with initials '{}' already exists in bid year {}",
                    initials.value(),
                    bid_year.year()
                )
            }
            Self::InvalidInitials(msg) => write!(f, "Invalid initials: {msg}"),
            Self::InvalidName(msg) => write!(f, "Invalid name: {msg}"),
            Self::InvalidArea(msg) => write!(f, "Invalid area: {msg}"),
            Self::InvalidCrew(msg) => write!(f, "Invalid crew: {msg}"),
            Self::InvalidUserType(msg) => write!(f, "Invalid user type: {msg}"),
            Self::BidYearNotFound(year) => write!(f, "Bid year {year} not found"),
            Self::AreaNotFound { bid_year, area } => {
                write!(f, "Area '{area}' not found in bid year {bid_year}")
            }
            Self::DuplicateBidYear(year) => write!(f, "Bid year {year} already exists"),
            Self::DuplicateArea { bid_year, area } => {
                write!(f, "Area '{area}' already exists in bid year {bid_year}")
            }
            Self::InvalidBidYear(msg) => write!(f, "Invalid bid year: {msg}"),
            Self::InvalidPayPeriodCount { count } => {
                write!(
                    f,
                    "Invalid pay period count: {count}. Must be exactly 26 or 27"
                )
            }
            Self::InvalidPayPeriodIndex { index, max } => {
                write!(
                    f,
                    "Invalid pay period index: {index}. Must be between 1 and {max}"
                )
            }
            Self::DateArithmeticOverflow { operation } => {
                write!(f, "Date arithmetic overflow while {operation}")
            }
            Self::InvalidStartDateWeekday {
                start_date,
                weekday,
            } => {
                write!(
                    f,
                    "Bid year start date must be a Sunday, but {start_date} is a {weekday}"
                )
            }
            Self::InvalidStartDateMonth { start_date, month } => {
                write!(
                    f,
                    "Bid year start date must be in January, but {start_date} is in {month}"
                )
            }
            Self::InvalidServiceComputationDate { reason } => {
                write!(f, "Invalid service computation date: {reason}")
            }
            Self::DateParseError { date_string, error } => {
                write!(f, "Failed to parse date '{date_string}': {error}")
            }
            Self::UserNotFound {
                bid_year,
                area,
                initials,
            } => {
                write!(
                    f,
                    "User with initials '{initials}' not found in area '{area}' for bid year {bid_year}"
                )
            }
            Self::MultipleBidYearsActive {
                current_active,
                requested_active,
            } => {
                write!(
                    f,
                    "Cannot set bid year {requested_active} as active: bid year {current_active} is already active"
                )
            }
            Self::NoActiveBidYear => {
                write!(f, "No active bid year is currently set")
            }
            Self::InvalidExpectedAreaCount { count } => {
                write!(
                    f,
                    "Invalid expected area count: {count}. Must be greater than 0"
                )
            }
            Self::InvalidExpectedUserCount { count } => {
                write!(
                    f,
                    "Invalid expected user count: {count}. Must be greater than 0"
                )
            }
            Self::CannotRemoveLastActiveAdmin => {
                write!(f, "Cannot disable or delete the last active admin operator")
            }
            Self::InvalidLifecycleState(state) => {
                write!(f, "Invalid lifecycle state: '{state}'")
            }
            Self::InvalidStateTransition { current, target } => {
                write!(f, "Invalid state transition from '{current}' to '{target}'")
            }
            Self::BootstrapIncomplete => {
                write!(
                    f,
                    "Cannot transition to BootstrapComplete: bootstrap is not marked complete"
                )
            }
            Self::AnotherBidYearAlreadyActive { active_year } => {
                write!(
                    f,
                    "Cannot activate bid year: year {active_year} is already active"
                )
            }
            Self::OperationNotAllowedInState { operation, state } => {
                write!(
                    f,
                    "Operation '{operation}' not allowed in lifecycle state '{state}'"
                )
            }
            Self::SystemAreaAlreadyExists { bid_year } => {
                write!(f, "System area already exists for bid year {bid_year}")
            }
            Self::UsersInNoBidArea {
                bid_year,
                user_count,
                sample_initials,
            } => {
                write!(
                    f,
                    "Cannot complete bootstrap for bid year {}: {} user(s) remain in No Bid area (sample: {})",
                    bid_year,
                    user_count,
                    sample_initials.join(", ")
                )
            }
            Self::CannotDeleteSystemArea { area_code } => {
                write!(f, "Cannot delete system area '{area_code}'")
            }
            Self::CannotRenameSystemArea { area_code } => {
                write!(f, "Cannot rename system area '{area_code}'")
            }
            Self::CannotEditAreaAfterCanonicalization {
                bid_year,
                lifecycle_state,
            } => {
                write!(
                    f,
                    "Cannot edit area metadata after canonicalization (bid year {bid_year}, state: {lifecycle_state})"
                )
            }
            Self::CannotDeleteUserAfterCanonicalization {
                bid_year,
                lifecycle_state,
            } => {
                write!(
                    f,
                    "Cannot delete user after canonicalization (bid year {bid_year}, state: {lifecycle_state})"
                )
            }
            Self::CannotAssignToNoBidAfterCanonicalization {
                bid_year,
                lifecycle_state,
            } => {
                write!(
                    f,
                    "Cannot assign user to No Bid area after canonicalization (bid year {bid_year}, state: {lifecycle_state})"
                )
            }
            Self::CannotOverrideBeforeCanonicalization { current_state } => {
                write!(
                    f,
                    "Cannot perform override before canonicalization (current state: {current_state})"
                )
            }
            Self::InvalidOverrideReason { reason } => {
                write!(
                    f,
                    "Invalid override reason: must be at least 10 characters (got: '{reason}')"
                )
            }
            Self::CanonicalRecordNotFound { description } => {
                write!(f, "Canonical record not found: {description}")
            }
            Self::CannotAssignToSystemArea { area_code } => {
                write!(f, "Cannot assign user to system area '{area_code}'")
            }
            Self::InvalidBidOrder { reason } => {
                write!(f, "Invalid bid order: {reason}")
            }
            Self::InvalidBidWindow { reason } => {
                write!(f, "Invalid bid window: {reason}")
            }
            Self::ParticipationFlagViolation {
                user_initials,
                reason,
            } => {
                write!(
                    f,
                    "Participation flag violation for user {user_initials}: {reason}"
                )
            }
            Self::RoundGroupNotFound { round_group_id } => {
                write!(f, "Round group with ID {round_group_id} not found")
            }
            Self::DuplicateRoundGroupName { bid_year, name } => {
                write!(
                    f,
                    "Round group with name '{name}' already exists in bid year {bid_year}"
                )
            }
            Self::RoundNotFound { round_id } => {
                write!(f, "Round with ID {round_id} not found")
            }
            Self::DuplicateRoundNumber {
                area_code,
                round_number,
            } => {
                write!(
                    f,
                    "Round number {round_number} already exists in area '{area_code}'"
                )
            }
            Self::CannotCreateRoundForSystemArea { area_code } => {
                write!(f, "Cannot create round for system area '{area_code}'")
            }
            Self::InvalidRoundConfiguration { reason } => {
                write!(f, "Invalid round configuration: {reason}")
            }
            Self::RoundGroupInUse {
                round_group_id,
                round_count,
            } => {
                write!(
                    f,
                    "Cannot delete round group {round_group_id}: referenced by {round_count} round(s)"
                )
            }
            Self::InvalidTimezone(tz) => {
                write!(f, "Invalid timezone identifier: '{tz}'")
            }
            Self::BidStartDateNotMonday(date) => {
                write!(
                    f,
                    "Bid start date must be a Monday, but {date} is a {}",
                    date.weekday()
                )
            }
            Self::BidStartDateNotFuture {
                start_date,
                reference_date,
            } => {
                write!(
                    f,
                    "Bid start date {start_date} must be in the future (after {reference_date})"
                )
            }
            Self::InvalidBidWindowTimes { start, end } => {
                write!(
                    f,
                    "Bid window start time ({start}) must be before end time ({end})"
                )
            }
            Self::InvalidBiddersPerDay(count) => {
                write!(f, "Bidders per day must be greater than 0, got {count}")
            }
        }
    }
}

impl std::error::Error for DomainError {}
