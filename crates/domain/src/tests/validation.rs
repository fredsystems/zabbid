// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    Area, BidYear, Crew, DomainError, Initials, SeniorityData, User, UserType, validate_bid_year,
    validate_initials_unique, validate_user_fields,
};

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
        Area::new("North"),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        false, // excluded_from_bidding
        false, // excluded_from_leave_calculation
        false, // no_bid_reviewed
    )
}

#[test]
fn test_validate_user_fields_accepts_valid_user() {
    let bid_year: BidYear = BidYear::new(2026);
    let initials: Initials = Initials::new("AB");
    let user: User = create_test_user(bid_year, initials);

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(result.is_ok());
}

#[test]
fn test_validate_user_fields_rejects_empty_initials() {
    let bid_year: BidYear = BidYear::new(2026);
    let initials: Initials = Initials::new("");
    let user: User = create_test_user(bid_year, initials);

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(matches!(result, Err(DomainError::InvalidInitials(_))));
}

#[test]
fn test_validate_user_fields_rejects_one_character_initials() {
    let bid_year: BidYear = BidYear::new(2026);
    let initials: Initials = Initials::new("A");
    let user: User = create_test_user(bid_year, initials);

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(matches!(result, Err(DomainError::InvalidInitials(_))));
}

#[test]
fn test_validate_user_fields_rejects_three_character_initials() {
    let bid_year: BidYear = BidYear::new(2026);
    let initials: Initials = Initials::new("ABC");
    let user: User = create_test_user(bid_year, initials);

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(matches!(result, Err(DomainError::InvalidInitials(_))));
}

#[test]
fn test_validate_user_fields_accepts_two_character_initials() {
    let bid_year: BidYear = BidYear::new(2026);
    let initials: Initials = Initials::new("AB");
    let user: User = create_test_user(bid_year, initials);

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(result.is_ok());
}

#[test]
fn test_validate_user_fields_rejects_empty_name() {
    let bid_year: BidYear = BidYear::new(2026);
    let user: User = User::new(
        bid_year,
        Initials::new("AB"),
        String::new(),
        Area::new("North"),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        false, // excluded_from_bidding
        false, // excluded_from_leave_calculation
        false, // no_bid_reviewed
    );

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(matches!(result, Err(DomainError::InvalidName(_))));
}

#[test]
fn test_validate_user_fields_rejects_empty_area() {
    let user: User = User::new(
        BidYear::new(2026),
        Initials::new("AB"),
        String::from("John Doe"),
        Area::new(""),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        false, // excluded_from_bidding
        false, // excluded_from_leave_calculation
        false, // no_bid_reviewed
    );

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(matches!(result, Err(DomainError::InvalidArea(_))));
}

#[test]
fn test_validate_user_fields_accepts_user_with_no_crew() {
    let user: User = User::new(
        BidYear::new(2026),
        Initials::new("AB"),
        String::from("John Doe"),
        Area::new("North"),
        UserType::CPC,
        None,
        create_test_seniority_data(),
        false, // excluded_from_bidding
        false, // excluded_from_leave_calculation
        false, // no_bid_reviewed
    );

    let result: Result<(), DomainError> = validate_user_fields(&user);
    assert!(result.is_ok());
}

#[test]
fn test_validate_bid_year_accepts_valid_years() {
    assert!(validate_bid_year(2026).is_ok());
    assert!(validate_bid_year(1900).is_ok());
    assert!(validate_bid_year(2200).is_ok());
}

#[test]
fn test_validate_bid_year_rejects_invalid_years() {
    assert!(matches!(
        validate_bid_year(1899),
        Err(DomainError::InvalidBidYear(_))
    ));
    assert!(matches!(
        validate_bid_year(2201),
        Err(DomainError::InvalidBidYear(_))
    ));
}

#[test]
fn test_validate_initials_unique_accepts_unique_initials() {
    let bid_year: BidYear = BidYear::new(2026);
    let existing_user: User = create_test_user(bid_year.clone(), Initials::new("AB"));
    let existing_users: Vec<User> = vec![existing_user];

    let new_initials: Initials = Initials::new("XY");
    let result: Result<(), DomainError> =
        validate_initials_unique(&bid_year, &new_initials, &existing_users);

    assert!(result.is_ok());
}

#[test]
fn test_validate_initials_unique_rejects_duplicate_initials() {
    let bid_year: BidYear = BidYear::new(2026);
    let existing_user: User = create_test_user(bid_year.clone(), Initials::new("AB"));
    let existing_users: Vec<User> = vec![existing_user];

    let duplicate_initials: Initials = Initials::new("AB");
    let result: Result<(), DomainError> =
        validate_initials_unique(&bid_year, &duplicate_initials, &existing_users);

    assert!(matches!(result, Err(DomainError::DuplicateInitials { .. })));
}

#[test]
fn test_validate_initials_unique_accepts_duplicate_in_different_bid_year() {
    let bid_year_2026: BidYear = BidYear::new(2026);
    let bid_year_2027: BidYear = BidYear::new(2027);

    let existing_user_2027: User = create_test_user(bid_year_2027, Initials::new("AB"));
    let existing_users: Vec<User> = vec![existing_user_2027];

    // Same initials but different bid year should be allowed
    let new_initials: Initials = Initials::new("AB");
    let result: Result<(), DomainError> =
        validate_initials_unique(&bid_year_2026, &new_initials, &existing_users);

    assert!(result.is_ok());
}

#[test]
fn test_validate_initials_unique_with_no_existing_users() {
    let bid_year: BidYear = BidYear::new(2026);
    let existing_users: Vec<User> = vec![];

    let new_initials: Initials = Initials::new("AB");
    let result: Result<(), DomainError> =
        validate_initials_unique(&bid_year, &new_initials, &existing_users);

    assert!(result.is_ok());
}

#[test]
fn test_validate_initials_unique_with_multiple_existing_users() {
    let bid_year: BidYear = BidYear::new(2026);
    let user1: User = create_test_user(bid_year.clone(), Initials::new("AB"));
    let user2: User = create_test_user(bid_year.clone(), Initials::new("CD"));
    let user3: User = create_test_user(bid_year.clone(), Initials::new("EF"));
    let existing_users: Vec<User> = vec![user1, user2, user3];

    // New unique initials should be accepted
    let new_initials: Initials = Initials::new("GH");
    let result: Result<(), DomainError> =
        validate_initials_unique(&bid_year, &new_initials, &existing_users);
    assert!(result.is_ok());

    // Duplicate initials should be rejected
    let duplicate_initials: Initials = Initials::new("CD");
    let result: Result<(), DomainError> =
        validate_initials_unique(&bid_year, &duplicate_initials, &existing_users);
    assert!(matches!(result, Err(DomainError::DuplicateInitials { .. })));
}

#[test]
fn test_validate_participation_flags_both_false() {
    let user: User = User::new(
        BidYear::new(2026),
        Initials::new("AB"),
        String::from("Test User"),
        Area::new("North"),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        false, // excluded_from_bidding
        false, // excluded_from_leave_calculation
        false, // no_bid_reviewed
    );

    let result: Result<(), DomainError> = user.validate_participation_flags();
    assert!(result.is_ok());
}

#[test]
fn test_validate_participation_flags_both_true() {
    let user: User = User::new(
        BidYear::new(2026),
        Initials::new("AB"),
        String::from("Test User"),
        Area::new("North"),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        true,  // excluded_from_bidding
        true,  // excluded_from_leave_calculation
        false, // no_bid_reviewed
    );

    let result: Result<(), DomainError> = user.validate_participation_flags();
    assert!(result.is_ok());
}

#[test]
fn test_validate_participation_flags_excluded_from_bidding_only() {
    let user: User = User::new(
        BidYear::new(2026),
        Initials::new("AB"),
        String::from("Test User"),
        Area::new("North"),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        true,  // excluded_from_bidding
        false, // excluded_from_leave_calculation
        false, // no_bid_reviewed
    );

    let result: Result<(), DomainError> = user.validate_participation_flags();
    assert!(result.is_ok());
}

#[test]
fn test_validate_participation_flags_invalid_excluded_from_leave_only() {
    let user: User = User::new(
        BidYear::new(2026),
        Initials::new("AB"),
        String::from("Test User"),
        Area::new("North"),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        false, // excluded_from_bidding
        true,  // excluded_from_leave_calculation
        false, // no_bid_reviewed
    );

    let result: Result<(), DomainError> = user.validate_participation_flags();
    assert!(matches!(
        result,
        Err(DomainError::ParticipationFlagViolation { .. })
    ));

    if let Err(DomainError::ParticipationFlagViolation {
        user_initials,
        reason,
    }) = result
    {
        assert_eq!(user_initials, "AB");
        assert!(reason.contains("excluded from leave calculation"));
        assert!(reason.contains("excluded from bidding"));
    }
}
