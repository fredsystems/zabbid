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
    clippy::all,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::unwrap_used,
    clippy::expect_used
)]

mod apply;
mod command;
mod error;
mod state;

#[cfg(test)]
mod tests;

use zab_bid_domain::{Area, BidYear, DomainError};

// Re-export public types and functions
pub use apply::{apply, apply_bootstrap};
pub use command::Command;
pub use error::CoreError;
pub use state::{BootstrapMetadata, BootstrapResult, State, TransitionResult};

/// Validates that a bid year exists in the metadata.
///
/// This is a read-only validation that does not create audit events.
///
/// # Arguments
///
/// * `metadata` - The bootstrap metadata to check
/// * `bid_year` - The bid year to validate
///
/// # Returns
///
/// * `Ok(())` if the bid year exists
/// * `Err(DomainError::BidYearNotFound)` if the bid year does not exist
///
/// # Errors
///
/// Returns an error if the bid year has not been created.
pub fn validate_bid_year_exists(
    metadata: &BootstrapMetadata,
    bid_year: &BidYear,
) -> Result<(), DomainError> {
    if !metadata.has_bid_year(bid_year) {
        return Err(DomainError::BidYearNotFound(bid_year.year()));
    }
    Ok(())
}

/// Validates that an area exists in the specified bid year.
///
/// This is a read-only validation that does not create audit events.
/// This function also validates that the bid year exists.
///
/// # Arguments
///
/// * `metadata` - The bootstrap metadata to check
/// * `bid_year` - The bid year to check within
/// * `area` - The area to validate
///
/// # Returns
///
/// * `Ok(())` if both the bid year and area exist
/// * `Err(DomainError::BidYearNotFound)` if the bid year does not exist
/// * `Err(DomainError::AreaNotFound)` if the area does not exist in the bid year
///
/// # Errors
///
/// Returns an error if:
/// - The bid year has not been created
/// - The area has not been created in the bid year
pub fn validate_area_exists(
    metadata: &BootstrapMetadata,
    bid_year: &BidYear,
    area: &Area,
) -> Result<(), DomainError> {
    // First validate bid year exists
    validate_bid_year_exists(metadata, bid_year)?;

    // Then validate area exists in that bid year
    if !metadata.has_area(bid_year, area) {
        return Err(DomainError::AreaNotFound {
            bid_year: bid_year.year(),
            area: area.id().to_string(),
        });
    }
    Ok(())
}
