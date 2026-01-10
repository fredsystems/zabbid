// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::error::DomainError;
use crate::types::{BidYear, Initials, User};
use std::collections::HashSet;

/// Validates that a user's basic field constraints are met.
///
/// This function checks that required fields are not empty.
/// It does NOT check for uniqueness (that requires context).
///
/// # Arguments
///
/// * `user` - The user to validate
///
/// # Returns
///
/// * `Ok(())` if the user's fields are valid
/// * `Err(DomainError)` if any field is invalid
///
/// # Errors
///
/// Returns an error if:
/// - The user's initials are empty
/// - The user's name is empty
/// - The user's area is empty
/// - The user's crew is empty
pub fn validate_user_fields(user: &User) -> Result<(), DomainError> {
    // Rule: initials must be exactly 2 characters
    let initials_len: usize = user.initials.value().len();
    if initials_len != 2 {
        return Err(DomainError::InvalidInitials(String::from(
            "Initials must be exactly 2 characters",
        )));
    }

    // Rule: name must not be empty
    if user.name.is_empty() {
        return Err(DomainError::InvalidName(String::from(
            "Name cannot be empty",
        )));
    }

    // Rule: area must not be empty
    if user.area.id().is_empty() {
        return Err(DomainError::InvalidArea(String::from(
            "Area cannot be empty",
        )));
    }

    // Crew validation is done at construction time via Crew::new()
    // No additional validation needed here since crew is optional

    Ok(())
}

/// Validates that a bid year is a valid calendar year.
///
/// # Arguments
///
/// * `year` - The year to validate
///
/// # Returns
///
/// * `Ok(())` if the year is valid
/// * `Err(DomainError::InvalidBidYear)` if the year is invalid
///
/// # Errors
///
/// Returns an error if the year is not a reasonable calendar year (1900-2200).
pub fn validate_bid_year(year: u16) -> Result<(), DomainError> {
    if !(1900..=2200).contains(&year) {
        return Err(DomainError::InvalidBidYear(format!(
            "Bid year must be between 1900 and 2200, got {year}"
        )));
    }
    Ok(())
}

/// Validates that user initials are unique within a bid year.
///
/// This is the representative domain rule for Phase 1.
/// This function is pure, deterministic, and has no side effects.
///
/// # Arguments
///
/// * `bid_year` - The bid year to check within
/// * `new_initials` - The initials to validate
/// * `existing_users` - The collection of existing users in the bid year
///
/// # Returns
///
/// * `Ok(())` if the initials are unique
/// * `Err(DomainError::DuplicateInitials)` if the initials already exist
///
/// # Errors
///
/// Returns an error if the initials are already in use within the bid year.
pub fn validate_initials_unique(
    bid_year: &BidYear,
    new_initials: &Initials,
    existing_users: &[User],
) -> Result<(), DomainError> {
    // Build a set of existing initials for this bid year
    let existing_initials: HashSet<&Initials> = existing_users
        .iter()
        .filter(|user| &user.bid_year == bid_year)
        .map(|user| &user.initials)
        .collect();

    // Rule: within a bid year, user initials must be unique
    if existing_initials.contains(new_initials) {
        return Err(DomainError::DuplicateInitials {
            bid_year: bid_year.clone(),
            initials: new_initials.clone(),
        });
    }

    Ok(())
}
