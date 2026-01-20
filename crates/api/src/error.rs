// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Error types for the API layer.

use crate::password_policy::PasswordPolicyError;
use zab_bid::CoreError;
#[allow(unused_imports)] // False positive: BidYear is used in pattern matching
use zab_bid_domain::{BidYear, DomainError};

/// Authentication and authorization errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Authentication failed.
    AuthenticationFailed {
        /// The reason authentication failed.
        reason: String,
    },
    /// Authorization failed.
    Unauthorized {
        /// The action that was attempted.
        action: String,
        /// The role required for this action.
        required_role: String,
    },
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {reason}")
            }
            Self::Unauthorized {
                action,
                required_role,
            } => {
                write!(f, "Unauthorized: '{action}' requires {required_role} role")
            }
        }
    }
}

impl std::error::Error for AuthError {}

/// API-level errors.
///
/// These are distinct from domain/core errors and represent the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// Authentication failed.
    AuthenticationFailed {
        /// The reason authentication failed.
        reason: String,
    },
    /// Authorization failed - the actor does not have permission.
    Unauthorized {
        /// The action that was attempted.
        action: String,
        /// The role required for this action.
        required_role: String,
    },
    /// A domain rule was violated.
    DomainRuleViolation {
        /// The rule that was violated.
        rule: String,
        /// A human-readable description of the violation.
        message: String,
    },
    /// Invalid input was provided.
    InvalidInput {
        /// The field that was invalid.
        field: String,
        /// A human-readable description of the error.
        message: String,
    },
    /// A requested resource was not found.
    ResourceNotFound {
        /// The type of resource that was not found.
        resource_type: String,
        /// A human-readable description of what was not found.
        message: String,
    },
    /// An internal error occurred.
    Internal {
        /// A description of the internal error.
        message: String,
    },
    /// Password policy violation.
    PasswordPolicyViolation {
        /// A human-readable description of the policy violation.
        message: String,
    },
    /// Invalid CSV format.
    InvalidCsvFormat {
        /// A human-readable description of the format error.
        reason: String,
    },
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {reason}")
            }
            Self::Unauthorized {
                action,
                required_role,
            } => {
                write!(f, "Unauthorized: '{action}' requires {required_role} role")
            }
            Self::DomainRuleViolation { rule, message } => {
                write!(f, "Domain rule violation ({rule}): {message}")
            }
            Self::InvalidInput { field, message } => {
                write!(f, "Invalid input for field '{field}': {message}")
            }
            Self::ResourceNotFound {
                resource_type,
                message,
            } => {
                write!(f, "{resource_type} not found: {message}")
            }
            Self::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
            Self::PasswordPolicyViolation { message } => {
                write!(f, "Password policy violation: {message}")
            }
            Self::InvalidCsvFormat { reason } => {
                write!(f, "Invalid CSV format: {reason}")
            }
        }
    }
}

impl std::error::Error for ApiError {}

impl From<AuthError> for ApiError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::AuthenticationFailed { reason } => Self::AuthenticationFailed { reason },
            AuthError::Unauthorized {
                action,
                required_role,
            } => Self::Unauthorized {
                action,
                required_role,
            },
        }
    }
}

impl From<PasswordPolicyError> for ApiError {
    fn from(err: PasswordPolicyError) -> Self {
        Self::PasswordPolicyViolation {
            message: err.to_string(),
        }
    }
}

/// Translates a domain error into an API error.
///
/// This translation is explicit and ensures domain errors are not leaked directly.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn translate_domain_error(err: DomainError) -> ApiError {
    match err {
        DomainError::DuplicateInitials { bid_year, initials } => ApiError::DomainRuleViolation {
            rule: String::from("unique_initials"),
            message: format!(
                "User with initials '{}' already exists in bid year {}",
                initials.value(),
                bid_year.year()
            ),
        },
        DomainError::InvalidInitials(msg) => ApiError::InvalidInput {
            field: String::from("initials"),
            message: msg,
        },
        DomainError::InvalidName(msg) => ApiError::InvalidInput {
            field: String::from("name"),
            message: msg,
        },
        DomainError::InvalidArea(msg) => ApiError::InvalidInput {
            field: String::from("area"),
            message: msg,
        },
        DomainError::InvalidCrew(msg) => ApiError::InvalidInput {
            field: String::from("crew"),
            message: msg.to_string(),
        },
        DomainError::InvalidUserType(msg) => ApiError::InvalidInput {
            field: String::from("user_type"),
            message: msg,
        },
        DomainError::BidYearNotFound(year) => ApiError::ResourceNotFound {
            resource_type: String::from("Bid year"),
            message: format!("Bid year {year} does not exist"),
        },
        DomainError::AreaNotFound { bid_year, area } => ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!("Area '{area}' does not exist in bid year {bid_year}"),
        },
        DomainError::DuplicateBidYear(year) => ApiError::DomainRuleViolation {
            rule: String::from("unique_bid_year"),
            message: format!("Bid year {year} already exists"),
        },
        DomainError::DuplicateArea { bid_year, area } => ApiError::DomainRuleViolation {
            rule: String::from("unique_area"),
            message: format!("Area '{area}' already exists in bid year {bid_year}"),
        },
        DomainError::InvalidBidYear(msg) => ApiError::InvalidInput {
            field: String::from("bid_year"),
            message: msg,
        },
        DomainError::InvalidPayPeriodCount { count } => ApiError::InvalidInput {
            field: String::from("pay_period_count"),
            message: format!("Invalid pay period count: {count}. Must be exactly 26 or 27"),
        },
        DomainError::InvalidPayPeriodIndex { index, max } => ApiError::InvalidInput {
            field: String::from("pay_period_index"),
            message: format!("Invalid pay period index: {index}. Must be between 1 and {max}"),
        },
        DomainError::DateArithmeticOverflow { operation } => ApiError::InvalidInput {
            field: String::from("date"),
            message: format!("Date arithmetic overflow while {operation}"),
        },
        DomainError::InvalidStartDateWeekday {
            start_date,
            weekday,
        } => ApiError::InvalidInput {
            field: String::from("start_date"),
            message: format!(
                "Bid year start date must be a Sunday, but {start_date} is a {weekday}"
            ),
        },
        DomainError::InvalidStartDateMonth { start_date, month } => ApiError::InvalidInput {
            field: String::from("start_date"),
            message: format!(
                "Bid year start date must be in January, but {start_date} is in {month}"
            ),
        },
        DomainError::InvalidServiceComputationDate { reason } => ApiError::InvalidInput {
            field: String::from("service_computation_date"),
            message: format!("Invalid service computation date: {reason}"),
        },
        DomainError::DateParseError { date_string, error } => ApiError::InvalidInput {
            field: String::from("date"),
            message: format!("Failed to parse date '{date_string}': {error}"),
        },
        DomainError::UserNotFound {
            bid_year,
            area,
            initials,
        } => ApiError::ResourceNotFound {
            resource_type: String::from("User"),
            message: format!(
                "User with initials '{initials}' not found in area '{area}' for bid year {bid_year}"
            ),
        },
        DomainError::MultipleBidYearsActive {
            current_active,
            requested_active,
        } => ApiError::DomainRuleViolation {
            rule: String::from("single_active_bid_year"),
            message: format!(
                "Cannot set bid year {requested_active} as active: bid year {current_active} is already active"
            ),
        },
        DomainError::NoActiveBidYear => ApiError::ResourceNotFound {
            resource_type: String::from("Active bid year"),
            message: String::from("No active bid year is currently set"),
        },
        DomainError::InvalidExpectedAreaCount { count } => ApiError::InvalidInput {
            field: String::from("expected_area_count"),
            message: format!("Invalid expected area count: {count}. Must be greater than 0"),
        },
        DomainError::InvalidExpectedUserCount { count } => ApiError::InvalidInput {
            field: String::from("expected_user_count"),
            message: format!("Invalid expected user count: {count}. Must be greater than 0"),
        },
        DomainError::CannotRemoveLastActiveAdmin => ApiError::DomainRuleViolation {
            rule: String::from("last_active_admin"),
            message: String::from("Operation would leave the system without an active admin"),
        },
        DomainError::InvalidLifecycleState(state) => ApiError::InvalidInput {
            field: String::from("lifecycle_state"),
            message: format!("Invalid lifecycle state: '{state}'"),
        },
        DomainError::InvalidStateTransition { current, target } => ApiError::DomainRuleViolation {
            rule: String::from("valid_lifecycle_transition"),
            message: format!("Invalid state transition from '{current}' to '{target}'"),
        },
        DomainError::BootstrapIncomplete => ApiError::DomainRuleViolation {
            rule: String::from("bootstrap_complete"),
            message: String::from(
                "Cannot transition to BootstrapComplete: bootstrap is not complete",
            ),
        },
        DomainError::AnotherBidYearAlreadyActive { active_year } => ApiError::DomainRuleViolation {
            rule: String::from("single_bidding_active_year"),
            message: format!(
                "Cannot activate bid year: year {active_year} is already in BiddingActive state"
            ),
        },
        DomainError::OperationNotAllowedInState { operation, state } => {
            ApiError::DomainRuleViolation {
                rule: String::from("operation_allowed_in_state"),
                message: format!(
                    "Operation '{operation}' not allowed in lifecycle state '{state}'"
                ),
            }
        }
        DomainError::SystemAreaAlreadyExists { bid_year } => ApiError::DomainRuleViolation {
            rule: String::from("system_area_uniqueness"),
            message: format!("System area already exists for bid year {bid_year}"),
        },
        DomainError::UsersInNoBidArea {
            bid_year,
            user_count,
            sample_initials,
        } => ApiError::DomainRuleViolation {
            rule: String::from("no_bid_area_empty"),
            message: format!(
                "Cannot complete bootstrap for bid year {}: {} user(s) remain in No Bid area ({})",
                bid_year,
                user_count,
                sample_initials.join(", ")
            ),
        },
        DomainError::CannotDeleteSystemArea { area_code } => ApiError::DomainRuleViolation {
            rule: String::from("system_area_immutable"),
            message: format!("Cannot delete system area '{area_code}'"),
        },
        DomainError::CannotRenameSystemArea { area_code } => ApiError::DomainRuleViolation {
            rule: String::from("system_area_immutable"),
            message: format!("Cannot rename system area '{area_code}'"),
        },
        DomainError::CannotEditAreaAfterCanonicalization {
            bid_year,
            lifecycle_state,
        } => ApiError::DomainRuleViolation {
            rule: String::from("no_area_edit_after_canonicalization"),
            message: format!(
                "Cannot edit area metadata after canonicalization (bid year {bid_year}, state: {lifecycle_state})"
            ),
        },
        DomainError::CannotDeleteUserAfterCanonicalization {
            bid_year,
            lifecycle_state,
        } => ApiError::DomainRuleViolation {
            rule: String::from("no_deletion_after_canonicalization"),
            message: format!(
                "Cannot delete user after canonicalization (bid year {bid_year}, state: {lifecycle_state})"
            ),
        },
        DomainError::CannotAssignToNoBidAfterCanonicalization {
            bid_year,
            lifecycle_state,
        } => ApiError::DomainRuleViolation {
            rule: String::from("no_assignment_to_no_bid_after_canonicalization"),
            message: format!(
                "Cannot assign user to No Bid area after canonicalization (bid year {bid_year}, state: {lifecycle_state})"
            ),
        },
        DomainError::CannotOverrideBeforeCanonicalization { current_state } => {
            ApiError::DomainRuleViolation {
                rule: String::from("override_requires_canonicalization"),
                message: format!(
                    "Cannot perform override before canonicalization (current state: {current_state})"
                ),
            }
        }
        DomainError::InvalidOverrideReason { reason } => ApiError::InvalidInput {
            field: String::from("reason"),
            message: format!(
                "Invalid override reason: must be at least 10 characters (got: '{reason}')"
            ),
        },
        DomainError::CanonicalRecordNotFound { description } => ApiError::ResourceNotFound {
            resource_type: String::from("Canonical record"),
            message: description,
        },
        DomainError::CannotAssignToSystemArea { area_code } => ApiError::DomainRuleViolation {
            rule: String::from("cannot_assign_to_system_area"),
            message: format!("Cannot assign user to system area '{area_code}'"),
        },
        DomainError::InvalidBidOrder { reason } => ApiError::InvalidInput {
            field: String::from("bid_order"),
            message: reason,
        },
        DomainError::InvalidBidWindow { reason } => ApiError::InvalidInput {
            field: String::from("bid_window"),
            message: reason,
        },
        DomainError::ParticipationFlagViolation {
            user_initials,
            reason,
        } => ApiError::DomainRuleViolation {
            rule: String::from("participation_flag_invariant"),
            message: format!("Participation flag violation for user '{user_initials}': {reason}"),
        },
        DomainError::RoundGroupNotFound { round_group_id } => ApiError::ResourceNotFound {
            resource_type: String::from("Round group"),
            message: format!("Round group with ID {round_group_id} not found"),
        },
        DomainError::DuplicateRoundGroupName { bid_year, name } => ApiError::DomainRuleViolation {
            rule: String::from("unique_round_group_name"),
            message: format!(
                "Round group with name '{name}' already exists in bid year {bid_year}"
            ),
        },
        DomainError::RoundNotFound { round_id } => ApiError::ResourceNotFound {
            resource_type: String::from("Round"),
            message: format!("Round with ID {round_id} not found"),
        },
        DomainError::DuplicateRoundNumber {
            area_code,
            round_number,
        } => ApiError::DomainRuleViolation {
            rule: String::from("unique_round_number"),
            message: format!("Round number {round_number} already exists in area '{area_code}'"),
        },
        DomainError::CannotCreateRoundForSystemArea { area_code } => {
            ApiError::DomainRuleViolation {
                rule: String::from("no_rounds_for_system_areas"),
                message: format!("Cannot create round for system area '{area_code}'"),
            }
        }
        DomainError::InvalidRoundConfiguration { reason } => ApiError::InvalidInput {
            field: String::from("round_configuration"),
            message: reason,
        },
        DomainError::RoundGroupInUse {
            round_group_id,
            round_count,
        } => ApiError::DomainRuleViolation {
            rule: String::from("round_group_in_use"),
            message: format!(
                "Cannot delete round group {round_group_id}: referenced by {round_count} round(s)"
            ),
        },
        DomainError::InvalidTimezone(tz) => ApiError::InvalidInput {
            field: String::from("timezone"),
            message: format!("Invalid timezone identifier: '{tz}'"),
        },
        DomainError::BidStartDateNotMonday(date) => ApiError::InvalidInput {
            field: String::from("start_date"),
            message: format!(
                "Bid start date must be a Monday, but {date} is a {}",
                date.weekday()
            ),
        },
        DomainError::BidStartDateNotFuture {
            start_date,
            reference_date,
        } => ApiError::InvalidInput {
            field: String::from("start_date"),
            message: format!(
                "Bid start date {start_date} must be in the future (after {reference_date})"
            ),
        },
        DomainError::InvalidBidWindowTimes { start, end } => ApiError::InvalidInput {
            field: String::from("bid_window"),
            message: format!("Bid window start time ({start}) must be before end time ({end})"),
        },
        DomainError::InvalidBiddersPerDay(count) => ApiError::InvalidInput {
            field: String::from("bidders_per_day"),
            message: format!("Bidders per day must be greater than 0, got {count}"),
        },
    }
}

/// Translates a core error into an API error.
///
/// This translation is explicit and ensures core errors are not leaked directly.
#[must_use]
pub fn translate_core_error(err: CoreError) -> ApiError {
    match err {
        CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        CoreError::Internal(msg) => ApiError::Internal {
            message: format!("Internal error: {msg}"),
        },
    }
}
