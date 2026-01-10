// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{Area, BidYear, Crew, DomainError, Initials, SeniorityData, User, UserType};

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
    )
}

#[test]
fn test_bid_year_creation() {
    let bid_year: BidYear = BidYear::new(2026);
    assert_eq!(bid_year.year(), 2026);
}

#[test]
fn test_initials_creation() {
    let initials: Initials = Initials::new("AB");
    assert_eq!(initials.value(), "AB");
}

#[test]
fn test_initials_normalized_to_uppercase() {
    let initials_lower: Initials = Initials::new("ab");
    let initials_mixed: Initials = Initials::new("Ab");
    let initials_upper: Initials = Initials::new("AB");

    assert_eq!(initials_lower.value(), "AB");
    assert_eq!(initials_mixed.value(), "AB");
    assert_eq!(initials_upper.value(), "AB");
}

#[test]
fn test_initials_case_insensitive_equality() {
    let initials_lower: Initials = Initials::new("ab");
    let initials_upper: Initials = Initials::new("AB");

    assert_eq!(initials_lower, initials_upper);
}

#[test]
fn test_area_creation() {
    let area: Area = Area::new("North");
    assert_eq!(area.id(), "NORTH");
}

#[test]
fn test_area_normalized_to_uppercase() {
    let area_lower: Area = Area::new("north");
    let area_mixed: Area = Area::new("North");
    let area_upper: Area = Area::new("NORTH");

    assert_eq!(area_lower.id(), "NORTH");
    assert_eq!(area_mixed.id(), "NORTH");
    assert_eq!(area_upper.id(), "NORTH");
}

#[test]
fn test_area_case_insensitive_equality() {
    let area_lower: Area = Area::new("north");
    let area_upper: Area = Area::new("NORTH");

    assert_eq!(area_lower, area_upper);
}

#[test]
fn test_crew_creation() {
    let crew: Result<Crew, DomainError> = Crew::new(1);
    assert!(crew.is_ok());
    assert_eq!(crew.unwrap().number(), 1);
}

#[test]
fn test_crew_validation_rejects_zero() {
    let crew: Result<Crew, DomainError> = Crew::new(0);
    assert!(matches!(crew, Err(DomainError::InvalidCrew(_))));
}

#[test]
fn test_crew_validation_rejects_eight() {
    let crew: Result<Crew, DomainError> = Crew::new(8);
    assert!(matches!(crew, Err(DomainError::InvalidCrew(_))));
}

#[test]
fn test_crew_validation_accepts_all_valid_values() {
    for n in 1..=7 {
        let crew: Result<Crew, DomainError> = Crew::new(n);
        assert!(crew.is_ok());
        assert_eq!(crew.unwrap().number(), n);
    }
}

#[test]
fn test_user_type_from_str() {
    assert_eq!(UserType::parse("CPC").unwrap(), UserType::CPC);
    assert_eq!(UserType::parse("CPC-IT").unwrap(), UserType::CpcIt);
    assert_eq!(UserType::parse("Dev-R").unwrap(), UserType::DevR);
    assert_eq!(UserType::parse("Dev-D").unwrap(), UserType::DevD);
}

#[test]
fn test_user_type_from_str_rejects_invalid() {
    let result: Result<UserType, DomainError> = UserType::parse("Invalid");
    assert!(matches!(result, Err(DomainError::InvalidUserType(_))));
}

#[test]
fn test_user_type_as_str() {
    assert_eq!(UserType::CPC.as_str(), "CPC");
    assert_eq!(UserType::CpcIt.as_str(), "CPC-IT");
    assert_eq!(UserType::DevR.as_str(), "Dev-R");
    assert_eq!(UserType::DevD.as_str(), "Dev-D");
}

#[test]
fn test_user_creation() {
    let bid_year: BidYear = BidYear::new(2026);
    let initials: Initials = Initials::new("AB");
    let user: User = create_test_user(bid_year.clone(), initials.clone());

    assert_eq!(user.bid_year, bid_year);
    assert_eq!(user.initials, initials);
    assert_eq!(user.name, "Test User");
    assert_eq!(user.area.id(), "NORTH");
    assert_eq!(user.user_type, UserType::CPC);
    assert!(user.crew.is_some());
    assert_eq!(user.crew.unwrap().number(), 1);
}
