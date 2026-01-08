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

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a bid year identifier.
///
/// A bid year is the scope within which users are identified and rules are enforced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BidYear {
    /// The year value (e.g., 2026).
    year: u16,
}

impl BidYear {
    /// Creates a new `BidYear`.
    ///
    /// # Arguments
    ///
    /// * `year` - The year value
    #[must_use]
    pub const fn new(year: u16) -> Self {
        Self { year }
    }

    /// Returns the year value.
    #[must_use]
    pub const fn year(&self) -> u16 {
        self.year
    }
}

/// Represents a user's initials.
///
/// Initials are the sole identifier for a user within a bid year.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Initials {
    /// The initials value (exactly 2 characters).
    value: String,
}

impl Initials {
    /// Creates new `Initials`.
    ///
    /// # Arguments
    ///
    /// * `value` - The initials value
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self { value }
    }

    /// Returns the initials value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Represents an area identifier.
///
/// A user must belong to exactly one area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Area {
    /// The area identifier (e.g., "North", "South").
    id: String,
}

impl Area {
    /// Creates a new `Area`.
    ///
    /// # Arguments
    ///
    /// * `id` - The area identifier
    #[must_use]
    pub const fn new(id: String) -> Self {
        Self { id }
    }

    /// Returns the area identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Represents a crew identifier.
///
/// A user must belong to exactly one crew.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crew {
    /// The crew identifier (e.g., "A", "B", "C").
    id: String,
}

impl Crew {
    /// Creates a new `Crew`.
    ///
    /// # Arguments
    ///
    /// * `id` - The crew identifier
    #[must_use]
    pub const fn new(id: String) -> Self {
        Self { id }
    }

    /// Returns the crew identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Represents seniority-related data for a user.
///
/// This data exists as domain data but must NOT be used for ordering,
/// ranking, or decision-making in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeniorityData {
    /// Cumulative NATCA bargaining unit date (ISO 8601 date string).
    pub cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date (ISO 8601 date string).
    pub natca_bu_date: String,
    /// Entry on Duty / FAA date (ISO 8601 date string).
    pub eod_faa_date: String,
    /// Service Computation Date (ISO 8601 date string).
    pub service_computation_date: String,
    /// Optional lottery value for tie-breaking (not used in Phase 1).
    pub lottery_value: Option<u32>,
}

impl SeniorityData {
    /// Creates new `SeniorityData`.
    ///
    /// # Arguments
    ///
    /// * `cumulative_natca_bu_date` - Cumulative NATCA bargaining unit date (ISO 8601 date)
    /// * `natca_bu_date` - NATCA bargaining unit date (ISO 8601 date)
    /// * `eod_faa_date` - Entry on Duty / FAA date
    /// * `service_computation_date` - Service Computation Date
    /// * `lottery_value` - Optional lottery value
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        cumulative_natca_bu_date: String,
        natca_bu_date: String,
        eod_faa_date: String,
        service_computation_date: String,
        lottery_value: Option<u32>,
    ) -> Self {
        Self {
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value,
        }
    }
}

/// Represents a user within a bid year.
///
/// Users are scoped to a single bid year and are uniquely identified
/// by their initials within that bid year.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// The bid year this user belongs to.
    pub bid_year: BidYear,
    /// The user's initials (sole identifier within the bid year).
    pub initials: Initials,
    /// The user's name (informational, not unique).
    pub name: String,
    /// The area this user belongs to.
    pub area: Area,
    /// The crew this user belongs to.
    pub crew: Crew,
    /// Seniority-related data (informational only in Phase 1).
    pub seniority_data: SeniorityData,
}

impl User {
    /// Creates a new `User`.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `initials` - The user's initials
    /// * `name` - The user's name
    /// * `area` - The user's area
    /// * `crew` - The user's crew
    /// * `seniority_data` - The user's seniority data
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        bid_year: BidYear,
        initials: Initials,
        name: String,
        area: Area,
        crew: Crew,
        seniority_data: SeniorityData,
    ) -> Self {
        Self {
            bid_year,
            initials,
            name,
            area,
            crew,
            seniority_data,
        }
    }
}

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
    InvalidCrew(String),
}

impl std::fmt::Display for DomainError {
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
        }
    }
}

impl std::error::Error for DomainError {}

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

    // Rule: crew must not be empty
    if user.crew.id().is_empty() {
        return Err(DomainError::InvalidCrew(String::from(
            "Crew cannot be empty",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_seniority_data() -> SeniorityData {
        SeniorityData::new(
            String::from("2019-01-15"),
            String::from("2019-06-01"),
            String::from("2020-01-15"),
            String::from("2020-01-15"),
            Some(42),
        )
    }

    fn create_test_user(bid_year: BidYear, initials: Initials) -> User {
        User::new(
            bid_year,
            initials,
            String::from("Test User"),
            Area::new(String::from("North")),
            Crew::new(String::from("A")),
            create_test_seniority_data(),
        )
    }

    #[test]
    fn test_bid_year_creation() {
        let bid_year: BidYear = BidYear::new(2026);
        assert_eq!(bid_year.year(), 2026);
    }

    #[test]
    fn test_initials_creation() {
        let initials: Initials = Initials::new(String::from("AB"));
        assert_eq!(initials.value(), "AB");
    }

    #[test]
    fn test_area_creation() {
        let area: Area = Area::new(String::from("North"));
        assert_eq!(area.id(), "North");
    }

    #[test]
    fn test_crew_creation() {
        let crew: Crew = Crew::new(String::from("A"));
        assert_eq!(crew.id(), "A");
    }

    #[test]
    fn test_user_creation() {
        let bid_year: BidYear = BidYear::new(2026);
        let initials: Initials = Initials::new(String::from("AB"));
        let user: User = create_test_user(bid_year.clone(), initials.clone());

        assert_eq!(user.bid_year, bid_year);
        assert_eq!(user.initials, initials);
        assert_eq!(user.name, "Test User");
        assert_eq!(user.area.id(), "North");
        assert_eq!(user.crew.id(), "A");
    }

    #[test]
    fn test_validate_user_fields_accepts_valid_user() {
        let bid_year: BidYear = BidYear::new(2026);
        let initials: Initials = Initials::new(String::from("AB"));
        let user: User = create_test_user(bid_year, initials);

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_user_fields_rejects_empty_initials() {
        let bid_year: BidYear = BidYear::new(2026);
        let initials: Initials = Initials::new(String::new());
        let user: User = create_test_user(bid_year, initials);

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(matches!(result, Err(DomainError::InvalidInitials(_))));
    }

    #[test]
    fn test_validate_user_fields_rejects_one_character_initials() {
        let bid_year: BidYear = BidYear::new(2026);
        let initials: Initials = Initials::new(String::from("A"));
        let user: User = create_test_user(bid_year, initials);

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(matches!(result, Err(DomainError::InvalidInitials(_))));
    }

    #[test]
    fn test_validate_user_fields_rejects_three_character_initials() {
        let bid_year: BidYear = BidYear::new(2026);
        let initials: Initials = Initials::new(String::from("ABC"));
        let user: User = create_test_user(bid_year, initials);

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(matches!(result, Err(DomainError::InvalidInitials(_))));
    }

    #[test]
    fn test_validate_user_fields_accepts_two_character_initials() {
        let bid_year: BidYear = BidYear::new(2026);
        let initials: Initials = Initials::new(String::from("AB"));
        let user: User = create_test_user(bid_year, initials);

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_user_fields_rejects_empty_name() {
        let user: User = User::new(
            BidYear::new(2026),
            Initials::new(String::from("AB")),
            String::new(),
            Area::new(String::from("North")),
            Crew::new(String::from("A")),
            create_test_seniority_data(),
        );

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(matches!(result, Err(DomainError::InvalidName(_))));
    }

    #[test]
    fn test_validate_user_fields_rejects_empty_area() {
        let user: User = User::new(
            BidYear::new(2026),
            Initials::new(String::from("AB")),
            String::from("John Doe"),
            Area::new(String::new()),
            Crew::new(String::from("A")),
            create_test_seniority_data(),
        );

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(matches!(result, Err(DomainError::InvalidArea(_))));
    }

    #[test]
    fn test_validate_user_fields_rejects_empty_crew() {
        let user: User = User::new(
            BidYear::new(2026),
            Initials::new(String::from("AB")),
            String::from("John Doe"),
            Area::new(String::from("North")),
            Crew::new(String::new()),
            create_test_seniority_data(),
        );

        let result: Result<(), DomainError> = validate_user_fields(&user);
        assert!(matches!(result, Err(DomainError::InvalidCrew(_))));
    }

    #[test]
    fn test_validate_initials_unique_accepts_unique_initials() {
        let bid_year: BidYear = BidYear::new(2026);
        let existing_user: User =
            create_test_user(bid_year.clone(), Initials::new(String::from("AB")));
        let existing_users: Vec<User> = vec![existing_user];

        let new_initials: Initials = Initials::new(String::from("XY"));
        let result: Result<(), DomainError> =
            validate_initials_unique(&bid_year, &new_initials, &existing_users);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_initials_unique_rejects_duplicate_initials() {
        let bid_year: BidYear = BidYear::new(2026);
        let existing_user: User =
            create_test_user(bid_year.clone(), Initials::new(String::from("AB")));
        let existing_users: Vec<User> = vec![existing_user];

        let duplicate_initials: Initials = Initials::new(String::from("AB"));
        let result: Result<(), DomainError> =
            validate_initials_unique(&bid_year, &duplicate_initials, &existing_users);

        assert!(matches!(result, Err(DomainError::DuplicateInitials { .. })));
    }

    #[test]
    fn test_validate_initials_unique_accepts_duplicate_in_different_bid_year() {
        let bid_year_2026: BidYear = BidYear::new(2026);
        let bid_year_2027: BidYear = BidYear::new(2027);

        let existing_user_2027: User =
            create_test_user(bid_year_2027, Initials::new(String::from("AB")));
        let existing_users: Vec<User> = vec![existing_user_2027];

        // Same initials but different bid year should be allowed
        let new_initials: Initials = Initials::new(String::from("AB"));
        let result: Result<(), DomainError> =
            validate_initials_unique(&bid_year_2026, &new_initials, &existing_users);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_initials_unique_with_no_existing_users() {
        let bid_year: BidYear = BidYear::new(2026);
        let existing_users: Vec<User> = vec![];

        let new_initials: Initials = Initials::new(String::from("AB"));
        let result: Result<(), DomainError> =
            validate_initials_unique(&bid_year, &new_initials, &existing_users);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_initials_unique_with_multiple_existing_users() {
        let bid_year: BidYear = BidYear::new(2026);
        let user1: User = create_test_user(bid_year.clone(), Initials::new(String::from("AB")));
        let user2: User = create_test_user(bid_year.clone(), Initials::new(String::from("CD")));
        let user3: User = create_test_user(bid_year.clone(), Initials::new(String::from("EF")));
        let existing_users: Vec<User> = vec![user1, user2, user3];

        // New unique initials should be accepted
        let new_initials: Initials = Initials::new(String::from("GH"));
        let result: Result<(), DomainError> =
            validate_initials_unique(&bid_year, &new_initials, &existing_users);
        assert!(result.is_ok());

        // Duplicate initials should be rejected
        let duplicate_initials: Initials = Initials::new(String::from("CD"));
        let result: Result<(), DomainError> =
            validate_initials_unique(&bid_year, &duplicate_initials, &existing_users);
        assert!(matches!(result, Err(DomainError::DuplicateInitials { .. })));
    }

    #[test]
    fn test_domain_error_display() {
        let bid_year: BidYear = BidYear::new(2026);
        let initials: Initials = Initials::new(String::from("AB"));

        let err: DomainError = DomainError::DuplicateInitials { bid_year, initials };
        assert_eq!(
            format!("{err}"),
            "User with initials 'AB' already exists in bid year 2026"
        );

        let err: DomainError = DomainError::InvalidInitials(String::from("test"));
        assert_eq!(format!("{err}"), "Invalid initials: test");

        let err: DomainError = DomainError::InvalidName(String::from("test"));
        assert_eq!(format!("{err}"), "Invalid name: test");

        let err: DomainError = DomainError::InvalidArea(String::from("test"));
        assert_eq!(format!("{err}"), "Invalid area: test");

        let err: DomainError = DomainError::InvalidCrew(String::from("test"));
        assert_eq!(format!("{err}"), "Invalid crew: test");
    }
}
