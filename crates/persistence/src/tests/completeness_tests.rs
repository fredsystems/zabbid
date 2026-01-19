// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for completeness tracking queries.
//!
//! These tests validate the counting and aggregation logic used to track
//! bootstrap completeness across bid years and areas.

use crate::SqlitePersistence;
use crate::tests::{
    create_test_actor, create_test_bid_year_and_area, create_test_cause, create_test_metadata,
    create_test_operator, create_test_seniority_data,
};
use zab_bid::{BootstrapMetadata, Command, State, apply, apply_bootstrap};
use zab_bid_domain::{Area, BidYear, Crew};

#[test]
fn test_count_users_by_area_empty() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    let bid_year_id = persistence.get_bid_year_id(2026).unwrap();
    let bid_year = BidYear::with_id(bid_year_id, 2026);
    let counts = persistence.count_users_by_area(&bid_year).unwrap();

    assert_eq!(
        counts.len(),
        0,
        "Should have no user counts when no users exist"
    );
}

#[test]
fn test_count_users_by_area_single_user() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Create a user via apply
    let state = State::new(BidYear::new(2026), Area::new("NORTH"));
    let cmd = Command::RegisterUser {
        initials: zab_bid_domain::Initials::new("AB"),
        name: String::from("Alice Bob"),
        area: Area::new("NORTH"),
        user_type: zab_bid_domain::UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let result = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();

    persistence.persist_transition(&result).unwrap();

    let bid_year_id = persistence.get_bid_year_id(2026).unwrap();
    let bid_year = BidYear::with_id(bid_year_id, 2026);
    let counts = persistence.count_users_by_area(&bid_year).unwrap();

    assert_eq!(counts.len(), 1, "Should have one area with users");
    assert_eq!(counts[0].0, "NORTH", "Area code should be NORTH");
    assert_eq!(counts[0].1, 1, "Should have 1 user in NORTH");
}

#[test]
fn test_count_users_by_area_multiple_users_single_area() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    let state = State::new(BidYear::new(2026), Area::new("NORTH"));

    // Create three users
    for (initials, name) in [
        ("AB", "Alice Bob"),
        ("CD", "Carol Dan"),
        ("EF", "Eve Frank"),
    ] {
        let cmd = Command::RegisterUser {
            initials: zab_bid_domain::Initials::new(initials),
            name: String::from(name),
            area: Area::new("NORTH"),
            user_type: zab_bid_domain::UserType::CPC,
            crew: Some(Crew::new(1).unwrap()),
            seniority_data: create_test_seniority_data(),
        };

        let result = apply(
            &create_test_metadata(),
            &state,
            &BidYear::new(2026),
            cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();

        persistence.persist_transition(&result).unwrap();
    }

    let bid_year_id = persistence.get_bid_year_id(2026).unwrap();
    let bid_year = BidYear::with_id(bid_year_id, 2026);
    let counts = persistence.count_users_by_area(&bid_year).unwrap();

    assert_eq!(counts.len(), 1, "Should have one area with users");
    assert_eq!(counts[0].0, "NORTH", "Area code should be NORTH");
    assert_eq!(counts[0].1, 3, "Should have 3 users in NORTH");
}

#[test]
fn test_count_users_by_area_multiple_areas() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Create second area
    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("NORTH")));

    let create_area_cmd = Command::CreateArea {
        area_id: String::from("SOUTH"),
    };
    let area_result = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        create_area_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&area_result).unwrap();

    // Update metadata with SOUTH area for user creation
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("SOUTH")));

    // Create users in different areas
    let users = [
        ("AB", "Alice Bob", "NORTH"),
        ("CD", "Carol Dan", "NORTH"),
        ("EF", "Eve Frank", "SOUTH"),
    ];

    for (initials, name, area) in users {
        let state = State::new(BidYear::new(2026), Area::new(area));
        let cmd = Command::RegisterUser {
            initials: zab_bid_domain::Initials::new(initials),
            name: String::from(name),
            area: Area::new(area),
            user_type: zab_bid_domain::UserType::CPC,
            crew: Some(Crew::new(1).unwrap()),
            seniority_data: create_test_seniority_data(),
        };

        let result = apply(
            &metadata,
            &state,
            &BidYear::new(2026),
            cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();

        persistence.persist_transition(&result).unwrap();
    }

    let bid_year_id = persistence.get_bid_year_id(2026).unwrap();
    let bid_year = BidYear::with_id(bid_year_id, 2026);
    let counts = persistence.count_users_by_area(&bid_year).unwrap();

    assert_eq!(counts.len(), 2, "Should have two areas with users");
    assert_eq!(counts[0].0, "NORTH", "First area should be NORTH (sorted)");
    assert_eq!(counts[0].1, 2, "NORTH should have 2 users");
    assert_eq!(counts[1].0, "SOUTH", "Second area should be SOUTH");
    assert_eq!(counts[1].1, 1, "SOUTH should have 1 user");
}

#[test]
fn test_count_areas_by_bid_year_empty() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let counts = persistence.count_areas_by_bid_year().unwrap();

    assert_eq!(
        counts.len(),
        0,
        "Should have no area counts when no bid years exist"
    );
}

#[test]
fn test_count_areas_by_bid_year_single_bid_year() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    let counts = persistence.count_areas_by_bid_year().unwrap();

    assert_eq!(counts.len(), 1, "Should have one bid year with areas");
    assert_eq!(counts[0].0, 2026, "Bid year should be 2026");
    assert_eq!(counts[0].1, 1, "Should have 1 area in 2026");
}

#[test]
fn test_count_areas_by_bid_year_multiple_areas() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Create additional areas
    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("NORTH")));

    for area_code in ["SOUTH", "EAST"] {
        let cmd = Command::CreateArea {
            area_id: String::from(area_code),
        };
        let result = apply_bootstrap(
            &metadata,
            &BidYear::new(2026),
            cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_bootstrap(&result).unwrap();
        metadata
            .areas
            .push((BidYear::new(2026), Area::new(area_code)));
    }

    let counts = persistence.count_areas_by_bid_year().unwrap();

    assert_eq!(counts.len(), 1, "Should have one bid year with areas");
    assert_eq!(counts[0].0, 2026, "Bid year should be 2026");
    assert_eq!(counts[0].1, 3, "Should have 3 areas in 2026");
}

#[test]
fn test_count_areas_by_bid_year_multiple_bid_years() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create two bid years with different numbers of areas
    create_test_bid_year_and_area(&mut persistence, 2025, "NORTH");
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Add another area to 2026
    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2025));
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2025), Area::new("NORTH")));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("NORTH")));

    let cmd = Command::CreateArea {
        area_id: String::from("SOUTH"),
    };
    let result = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result).unwrap();

    let counts = persistence.count_areas_by_bid_year().unwrap();

    assert_eq!(counts.len(), 2, "Should have two bid years with areas");
    assert_eq!(counts[0].0, 2025, "First bid year should be 2025 (sorted)");
    assert_eq!(counts[0].1, 1, "2025 should have 1 area");
    assert_eq!(counts[1].0, 2026, "Second bid year should be 2026");
    assert_eq!(counts[1].1, 2, "2026 should have 2 areas");
}

#[test]
fn test_count_users_by_bid_year_empty() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let counts = persistence.count_users_by_bid_year().unwrap();

    assert_eq!(
        counts.len(),
        0,
        "Should have no user counts when no bid years exist"
    );
}

#[test]
fn test_count_users_by_bid_year_single_bid_year() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Create a user
    let state = State::new(BidYear::new(2026), Area::new("NORTH"));
    let cmd = Command::RegisterUser {
        initials: zab_bid_domain::Initials::new("AB"),
        name: String::from("Alice Bob"),
        area: Area::new("NORTH"),
        user_type: zab_bid_domain::UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let result = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();

    persistence.persist_transition(&result).unwrap();

    let counts = persistence.count_users_by_bid_year().unwrap();

    assert_eq!(counts.len(), 1, "Should have one bid year with users");
    assert_eq!(counts[0].0, 2026, "Bid year should be 2026");
    assert_eq!(counts[0].1, 1, "Should have 1 user in 2026");
}

#[test]
fn test_count_users_by_bid_year_multiple_bid_years() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create two bid years
    create_test_bid_year_and_area(&mut persistence, 2025, "NORTH");
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Build metadata with both bid years
    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2025));
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2025), Area::new("NORTH")));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("NORTH")));

    // Create users in different bid years
    let users = [
        ("AB", "Alice Bob", 2025),
        ("CD", "Carol Dan", 2026),
        ("EF", "Eve Frank", 2026),
    ];

    for (initials, name, year) in users {
        let state = State::new(BidYear::new(year), Area::new("NORTH"));
        let cmd = Command::RegisterUser {
            initials: zab_bid_domain::Initials::new(initials),
            name: String::from(name),
            area: Area::new("NORTH"),
            user_type: zab_bid_domain::UserType::CPC,
            crew: Some(Crew::new(1).unwrap()),
            seniority_data: create_test_seniority_data(),
        };

        let result = apply(
            &metadata,
            &state,
            &BidYear::new(year),
            cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();

        persistence.persist_transition(&result).unwrap();
    }

    let counts = persistence.count_users_by_bid_year().unwrap();

    assert_eq!(counts.len(), 2, "Should have two bid years with users");
    assert_eq!(counts[0].0, 2025, "First bid year should be 2025 (sorted)");
    assert_eq!(counts[0].1, 1, "2025 should have 1 user");
    assert_eq!(counts[1].0, 2026, "Second bid year should be 2026");
    assert_eq!(counts[1].1, 2, "2026 should have 2 users");
}

#[test]
fn test_count_users_by_bid_year_and_area_empty() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let counts = persistence.count_users_by_bid_year_and_area().unwrap();

    assert_eq!(counts.len(), 0, "Should have no counts when no data exists");
}

#[test]
fn test_count_users_by_bid_year_and_area_single_combination() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Create a user
    let state = State::new(BidYear::new(2026), Area::new("NORTH"));
    let cmd = Command::RegisterUser {
        initials: zab_bid_domain::Initials::new("AB"),
        name: String::from("Alice Bob"),
        area: Area::new("NORTH"),
        user_type: zab_bid_domain::UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let result = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();

    persistence.persist_transition(&result).unwrap();

    let counts = persistence.count_users_by_bid_year_and_area().unwrap();

    assert_eq!(counts.len(), 1, "Should have one combination");
    assert_eq!(counts[0].0, 2026, "Bid year should be 2026");
    assert_eq!(counts[0].1, "NORTH", "Area should be NORTH");
    assert_eq!(counts[0].2, 1, "Should have 1 user");
}

#[test]
fn test_count_users_by_bid_year_and_area_multiple_combinations() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create two bid years with multiple areas
    create_test_bid_year_and_area(&mut persistence, 2025, "NORTH");
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2025));
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2025), Area::new("NORTH")));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("NORTH")));

    // Create SOUTH area for 2026
    let cmd = Command::CreateArea {
        area_id: String::from("SOUTH"),
    };
    let result = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result).unwrap();

    // Update metadata with SOUTH area
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("SOUTH")));

    // Create users in different combinations
    let users = [
        ("AB", "Alice Bob", 2025, "NORTH"),
        ("CD", "Carol Dan", 2025, "NORTH"),
        ("EF", "Eve Frank", 2026, "NORTH"),
        ("GH", "George Helen", 2026, "SOUTH"),
        ("IJ", "Ivan Jane", 2026, "SOUTH"),
        ("KL", "Karl Lisa", 2026, "SOUTH"),
    ];

    for (initials, name, year, area) in users {
        let state = State::new(BidYear::new(year), Area::new(area));
        let cmd = Command::RegisterUser {
            initials: zab_bid_domain::Initials::new(initials),
            name: String::from(name),
            area: Area::new(area),
            user_type: zab_bid_domain::UserType::CPC,
            crew: Some(Crew::new(1).unwrap()),
            seniority_data: create_test_seniority_data(),
        };

        let result = apply(
            &metadata,
            &state,
            &BidYear::new(year),
            cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();

        persistence.persist_transition(&result).unwrap();
    }

    let counts = persistence.count_users_by_bid_year_and_area().unwrap();

    assert_eq!(counts.len(), 3, "Should have three combinations");

    // Verify sorted order (by year, then area)
    assert_eq!(counts[0].0, 2025, "First should be 2025");
    assert_eq!(counts[0].1, "NORTH", "First area should be NORTH");
    assert_eq!(counts[0].2, 2, "2025/NORTH should have 2 users");

    assert_eq!(counts[1].0, 2026, "Second should be 2026");
    assert_eq!(counts[1].1, "NORTH", "Second area should be NORTH (sorted)");
    assert_eq!(counts[1].2, 1, "2026/NORTH should have 1 user");

    assert_eq!(counts[2].0, 2026, "Third should be 2026");
    assert_eq!(counts[2].1, "SOUTH", "Third area should be SOUTH");
    assert_eq!(counts[2].2, 3, "2026/SOUTH should have 3 users");
}

#[test]
fn test_count_users_by_area_filters_by_bid_year() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create two bid years with same area code
    create_test_bid_year_and_area(&mut persistence, 2025, "NORTH");
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Build metadata with both bid years
    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2025));
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2025), Area::new("NORTH")));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("NORTH")));

    // Create users in both years
    for (initials, name, year) in [("AB", "Alice Bob", 2025), ("CD", "Carol Dan", 2026)] {
        let state = State::new(BidYear::new(year), Area::new("NORTH"));
        let cmd = Command::RegisterUser {
            initials: zab_bid_domain::Initials::new(initials),
            name: String::from(name),
            area: Area::new("NORTH"),
            user_type: zab_bid_domain::UserType::CPC,
            crew: Some(Crew::new(1).unwrap()),
            seniority_data: create_test_seniority_data(),
        };

        let result = apply(
            &metadata,
            &state,
            &BidYear::new(year),
            cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();

        persistence.persist_transition(&result).unwrap();
    }

    // Count for 2026 only
    let bid_year_id = persistence.get_bid_year_id(2026).unwrap();
    let bid_year = BidYear::with_id(bid_year_id, 2026);
    let counts = persistence.count_users_by_area(&bid_year).unwrap();

    assert_eq!(counts.len(), 1, "Should have one area");
    assert_eq!(counts[0].0, "NORTH", "Area should be NORTH");
    assert_eq!(
        counts[0].1, 1,
        "Should count only users from 2026, not 2025"
    );
}

#[test]
fn test_count_conversion_handles_zero() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // No users created - counts should be zero but conversion should succeed
    let counts = persistence.count_areas_by_bid_year().unwrap();
    assert_eq!(counts.len(), 1, "Should have bid year entry");
    assert_eq!(counts[0].1, 1, "Should have 1 area");

    let user_counts = persistence.count_users_by_bid_year().unwrap();
    assert_eq!(
        user_counts.len(),
        0,
        "Should have no entries when no users exist"
    );
}
