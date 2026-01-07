// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![deny(
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]

/// Represents a request to submit a bid.
///
/// This is a minimal domain type for Phase 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidRequest {
    /// The unique identifier of the employee making the bid.
    pub employee_id: String,
    /// The period being bid for (e.g., "2026-Q1").
    pub period: String,
    /// The requested days (minimal representation for Phase 0).
    pub requested_days: Vec<String>,
}

/// Errors that can occur during domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// The employee ID is empty or invalid.
    InvalidEmployeeId(String),
    /// The period is empty or invalid.
    InvalidPeriod(String),
    /// The requested days list is empty.
    NoRequestedDays,
    /// The requested days list contains duplicates.
    DuplicateRequestedDays,
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEmployeeId(msg) => write!(f, "Invalid employee ID: {msg}"),
            Self::InvalidPeriod(msg) => write!(f, "Invalid period: {msg}"),
            Self::NoRequestedDays => write!(f, "Requested days cannot be empty"),
            Self::DuplicateRequestedDays => write!(f, "Requested days contain duplicates"),
        }
    }
}

impl std::error::Error for DomainError {}

/// Validates a bid request according to domain rules.
///
/// This function is pure, deterministic, and has no side effects.
///
/// # Arguments
///
/// * `req` - The bid request to validate
///
/// # Returns
///
/// * `Ok(())` if the bid request is valid
/// * `Err(DomainError)` if the bid request violates domain rules
///
/// # Errors
///
/// Returns an error if:
/// - The employee ID is empty
/// - The period is empty
/// - The requested days list is empty
/// - The requested days list contains duplicates
pub fn validate_bid(req: &BidRequest) -> Result<(), DomainError> {
    // Rule: employee_id must not be empty
    if req.employee_id.is_empty() {
        return Err(DomainError::InvalidEmployeeId(String::from(
            "Employee ID cannot be empty",
        )));
    }

    // Rule: period must not be empty
    if req.period.is_empty() {
        return Err(DomainError::InvalidPeriod(String::from(
            "Period cannot be empty",
        )));
    }

    // Rule: requested_days must not be empty
    if req.requested_days.is_empty() {
        return Err(DomainError::NoRequestedDays);
    }

    // Rule: requested_days must not contain duplicates
    let mut sorted_days: Vec<String> = req.requested_days.clone();
    sorted_days.sort();
    for i in 1..sorted_days.len() {
        if sorted_days[i] == sorted_days[i - 1] {
            return Err(DomainError::DuplicateRequestedDays);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bid_request() {
        let req: BidRequest = BidRequest {
            employee_id: String::from("EMP001"),
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15"), String::from("2026-01-16")],
        };

        let result: Result<(), DomainError> = validate_bid(&req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_employee_id_rejected() {
        let req: BidRequest = BidRequest {
            employee_id: String::new(),
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };

        let result: Result<(), DomainError> = validate_bid(&req);
        assert!(matches!(result, Err(DomainError::InvalidEmployeeId(_))));
    }

    #[test]
    fn test_empty_period_rejected() {
        let req: BidRequest = BidRequest {
            employee_id: String::from("EMP001"),
            period: String::new(),
            requested_days: vec![String::from("2026-01-15")],
        };

        let result: Result<(), DomainError> = validate_bid(&req);
        assert!(matches!(result, Err(DomainError::InvalidPeriod(_))));
    }

    #[test]
    fn test_empty_requested_days_rejected() {
        let req: BidRequest = BidRequest {
            employee_id: String::from("EMP001"),
            period: String::from("2026-Q1"),
            requested_days: vec![],
        };

        let result: Result<(), DomainError> = validate_bid(&req);
        assert!(matches!(result, Err(DomainError::NoRequestedDays)));
    }

    #[test]
    fn test_duplicate_requested_days_rejected() {
        let req: BidRequest = BidRequest {
            employee_id: String::from("EMP001"),
            period: String::from("2026-Q1"),
            requested_days: vec![
                String::from("2026-01-15"),
                String::from("2026-01-16"),
                String::from("2026-01-15"),
            ],
        };

        let result: Result<(), DomainError> = validate_bid(&req);
        assert!(matches!(result, Err(DomainError::DuplicateRequestedDays)));
    }

    #[test]
    fn test_domain_error_display() {
        let err: DomainError = DomainError::InvalidEmployeeId(String::from("test"));
        assert_eq!(format!("{err}"), "Invalid employee ID: test");

        let err: DomainError = DomainError::InvalidPeriod(String::from("test"));
        assert_eq!(format!("{err}"), "Invalid period: test");

        let err: DomainError = DomainError::NoRequestedDays;
        assert_eq!(format!("{err}"), "Requested days cannot be empty");

        let err: DomainError = DomainError::DuplicateRequestedDays;
        assert_eq!(format!("{err}"), "Requested days contain duplicates");
    }
}
