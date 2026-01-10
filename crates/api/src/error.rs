// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Error types for the API layer.

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

/// Translates a domain error into an API error.
///
/// This translation is explicit and ensures domain errors are not leaked directly.
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
        DomainError::BidYearNotFound(year) => ApiError::DomainRuleViolation {
            rule: String::from("bid_year_exists"),
            message: format!("Bid year {year} not found"),
        },
        DomainError::AreaNotFound { bid_year, area } => ApiError::DomainRuleViolation {
            rule: String::from("area_exists"),
            message: format!("Area '{area}' not found in bid year {bid_year}"),
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
    }
}
