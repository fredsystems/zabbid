// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    Area, BidYear, Crew, DomainError, Initials, Round, RoundGroup, SeniorityData, User, UserType,
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

// ============================================================================
// Gap 10: Domain Validation Error Messages
// ============================================================================

/// `PHASE_27H.10`: Test initials with empty string
#[test]
fn test_initials_empty_string() {
    let initials: Initials = Initials::new("");
    assert_eq!(initials.value(), "");
}

/// `PHASE_27H.10`: Test initials with single character
#[test]
fn test_initials_single_character() {
    let initials: Initials = Initials::new("A");
    assert_eq!(initials.value(), "A");
}

/// `PHASE_27H.10`: Test initials with more than two characters
#[test]
fn test_initials_three_characters() {
    let initials: Initials = Initials::new("ABC");
    assert_eq!(initials.value(), "ABC");
}

/// `PHASE_27H.10`: Test initials with special characters
#[test]
fn test_initials_with_special_characters() {
    let initials: Initials = Initials::new("A-B");
    assert_eq!(initials.value(), "A-B");
}

/// `PHASE_27H.10`: Test initials with numbers
#[test]
fn test_initials_with_numbers() {
    let initials: Initials = Initials::new("A1");
    assert_eq!(initials.value(), "A1");
}

/// `PHASE_27H.10`: Test initials with whitespace
#[test]
fn test_initials_with_whitespace() {
    let initials: Initials = Initials::new("A B");
    assert_eq!(initials.value(), "A B");
}

/// `PHASE_27H.10`: Test crew validation with maximum u8 value
#[test]
fn test_crew_validation_rejects_max_u8() {
    let crew: Result<Crew, DomainError> = Crew::new(255);
    assert!(matches!(crew, Err(DomainError::InvalidCrew(_))));
}

/// `PHASE_27H.10`: Test crew error message contains helpful information
#[test]
fn test_crew_error_message_is_descriptive() {
    let crew_result: Result<Crew, DomainError> = Crew::new(0);
    assert!(crew_result.is_err());

    if let Err(DomainError::InvalidCrew(msg)) = crew_result {
        assert!(msg.contains('1'));
        assert!(msg.contains('7'));
    } else {
        panic!("Expected InvalidCrew error");
    }
}

/// `PHASE_27H.10`: Test user type parsing is case-sensitive
#[test]
fn test_user_type_parse_is_case_sensitive() {
    assert!(UserType::parse("cpc").is_err());
    assert!(UserType::parse("Cpc").is_err());
    assert!(UserType::parse("cpc-it").is_err());
}

/// `PHASE_27H.10`: Test user type error message contains the invalid value
#[test]
fn test_user_type_error_message_contains_input() {
    let result: Result<UserType, DomainError> = UserType::parse("InvalidType");
    assert!(result.is_err());

    if let Err(DomainError::InvalidUserType(msg)) = result {
        assert!(msg.contains("InvalidType"));
    } else {
        panic!("Expected InvalidUserType error");
    }
}

/// `PHASE_27H.10`: Test user type with empty string
#[test]
fn test_user_type_parse_empty_string() {
    let result: Result<UserType, DomainError> = UserType::parse("");
    assert!(matches!(result, Err(DomainError::InvalidUserType(_))));
}

/// `PHASE_27H.10`: Test user type with whitespace
#[test]
fn test_user_type_parse_whitespace() {
    let result: Result<UserType, DomainError> = UserType::parse(" CPC ");
    assert!(matches!(result, Err(DomainError::InvalidUserType(_))));
}

/// `PHASE_27H.10`: Test user type with similar but invalid values
#[test]
fn test_user_type_parse_similar_invalid_values() {
    assert!(UserType::parse("CPC_IT").is_err());
    assert!(UserType::parse("CPCIT").is_err());
    assert!(UserType::parse("Dev R").is_err());
    assert!(UserType::parse("DevR").is_err());
}

/// `PHASE_27H.10`: Test area code normalization with special characters
#[test]
fn test_area_code_with_special_characters() {
    let area: Area = Area::new("NORTH-EAST");
    assert_eq!(area.id(), "NORTH-EAST");
}

/// `PHASE_27H.10`: Test area code with numbers
#[test]
fn test_area_code_with_numbers() {
    let area: Area = Area::new("AREA1");
    assert_eq!(area.id(), "AREA1");
}

/// `PHASE_27H.10`: Test area code with empty string
#[test]
fn test_area_code_empty_string() {
    let area: Area = Area::new("");
    assert_eq!(area.id(), "");
}

/// `PHASE_27H.10`: Test area code with whitespace
#[test]
fn test_area_code_with_whitespace() {
    let area: Area = Area::new("NORTH AREA");
    assert_eq!(area.id(), "NORTH AREA");
}

/// `PHASE_27H.10`: Test crew boundary values 1 and 7
#[test]
fn test_crew_boundary_values() {
    let crew1: Result<Crew, DomainError> = Crew::new(1);
    let crew7: Result<Crew, DomainError> = Crew::new(7);

    assert!(crew1.is_ok());
    assert!(crew7.is_ok());
    assert_eq!(crew1.unwrap().number(), 1);
    assert_eq!(crew7.unwrap().number(), 7);
}

// ============================================================================
// Phase 29B: Round and RoundGroup Validation Tests
// ============================================================================

/// Helper to create a test bid year
fn create_test_bid_year() -> BidYear {
    BidYear::new(2026)
}

/// Helper to create a test round group
fn create_test_round_group() -> RoundGroup {
    RoundGroup::new(create_test_bid_year(), String::from("Regular Round"), true)
}

/// Helper to create a test round
fn create_test_round() -> Round {
    Round::new(
        create_test_round_group(),
        1,
        String::from("Round 1"),
        10,    // slots_per_day
        5,     // max_groups
        80,    // max_total_hours
        false, // include_holidays
        false, // allow_overbid
    )
}

// RoundGroup validation tests

#[test]
fn test_round_group_validate_constraints_accepts_valid_name() {
    let round_group = create_test_round_group();
    assert!(round_group.validate_constraints().is_ok());
}

#[test]
fn test_round_group_validate_constraints_rejects_empty_name() {
    let round_group = RoundGroup::new(create_test_bid_year(), String::new(), true);
    let result = round_group.validate_constraints();
    assert!(matches!(
        result,
        Err(DomainError::InvalidRoundConfiguration { .. })
    ));
    if let Err(DomainError::InvalidRoundConfiguration { reason }) = result {
        assert!(reason.contains("name cannot be empty"));
    }
}

#[test]
fn test_round_group_validate_constraints_rejects_whitespace_only_name() {
    let round_group = RoundGroup::new(create_test_bid_year(), String::from("   "), true);
    let result = round_group.validate_constraints();
    assert!(matches!(
        result,
        Err(DomainError::InvalidRoundConfiguration { .. })
    ));
    if let Err(DomainError::InvalidRoundConfiguration { reason }) = result {
        assert!(reason.contains("name cannot be empty"));
    }
}

#[test]
fn test_round_group_validate_constraints_accepts_name_with_leading_trailing_whitespace() {
    let round_group = RoundGroup::new(create_test_bid_year(), String::from("  Valid Name  "), true);
    // Name with whitespace around actual content should pass
    // (trimming is done in validation, not storage)
    assert!(round_group.validate_constraints().is_ok());
}

// Round validation tests

#[test]
fn test_round_validate_constraints_accepts_valid_configuration() {
    let round = create_test_round();
    assert!(round.validate_constraints().is_ok());
}

#[test]
fn test_round_validate_constraints_rejects_zero_slots_per_day() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::from("Round 1"),
        0, // slots_per_day = 0
        5,
        80,
        false,
        false,
    );
    let result = round.validate_constraints();
    assert!(matches!(
        result,
        Err(DomainError::InvalidRoundConfiguration { .. })
    ));
    if let Err(DomainError::InvalidRoundConfiguration { reason }) = result {
        assert!(reason.contains("slots_per_day must be greater than 0"));
    }
}

#[test]
fn test_round_validate_constraints_rejects_zero_max_groups() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::from("Round 1"),
        10,
        0, // max_groups = 0
        80,
        false,
        false,
    );
    let result = round.validate_constraints();
    assert!(matches!(
        result,
        Err(DomainError::InvalidRoundConfiguration { .. })
    ));
    if let Err(DomainError::InvalidRoundConfiguration { reason }) = result {
        assert!(reason.contains("max_groups must be greater than 0"));
    }
}

#[test]
fn test_round_validate_constraints_rejects_zero_max_total_hours() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::from("Round 1"),
        10,
        5,
        0, // max_total_hours = 0
        false,
        false,
    );
    let result = round.validate_constraints();
    assert!(matches!(
        result,
        Err(DomainError::InvalidRoundConfiguration { .. })
    ));
    if let Err(DomainError::InvalidRoundConfiguration { reason }) = result {
        assert!(reason.contains("max_total_hours must be greater than 0"));
    }
}

#[test]
fn test_round_validate_constraints_rejects_empty_name() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::new(),
        10,
        5,
        80,
        false,
        false,
    );
    let result = round.validate_constraints();
    assert!(matches!(
        result,
        Err(DomainError::InvalidRoundConfiguration { .. })
    ));
    if let Err(DomainError::InvalidRoundConfiguration { reason }) = result {
        assert!(reason.contains("name cannot be empty"));
    }
}

#[test]
fn test_round_validate_constraints_rejects_whitespace_only_name() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::from("   "),
        10,
        5,
        80,
        false,
        false,
    );
    let result = round.validate_constraints();
    assert!(matches!(
        result,
        Err(DomainError::InvalidRoundConfiguration { .. })
    ));
    if let Err(DomainError::InvalidRoundConfiguration { reason }) = result {
        assert!(reason.contains("name cannot be empty"));
    }
}

#[test]
fn test_round_validate_constraints_accepts_minimum_valid_values() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::from("Minimal Round"),
        1, // slots_per_day = 1 (minimum valid)
        1, // max_groups = 1 (minimum valid)
        1, // max_total_hours = 1 (minimum valid)
        false,
        false,
    );
    assert!(round.validate_constraints().is_ok());
}

#[test]
fn test_round_with_overbid_allowed() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::from("Carryover Round"),
        10,
        5,
        80,
        false,
        true, // allow_overbid = true
    );
    assert!(round.allow_overbid());
    assert!(round.validate_constraints().is_ok());
}

#[test]
fn test_round_with_holidays_included() {
    let round = Round::new(
        create_test_round_group(),
        1,
        String::from("Holiday Round"),
        10,
        5,
        80,
        true, // include_holidays = true
        false,
    );
    assert!(round.include_holidays());
    assert!(round.validate_constraints().is_ok());
}

#[test]
fn test_round_group_with_editing_disabled() {
    let round_group = RoundGroup::new(
        create_test_bid_year(),
        String::from("Locked Round Group"),
        false, // editing_enabled = false
    );
    assert!(!round_group.editing_enabled());
    assert!(round_group.validate_constraints().is_ok());
}
