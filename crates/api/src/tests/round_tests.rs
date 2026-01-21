// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Integration tests for round groups and rounds API endpoints (Phase 29B).

use crate::{
    ApiError, AssignAreaRoundGroupRequest, AuthenticatedActor, CreateRoundGroupRequest,
    CreateRoundRequest, UpdateRoundGroupRequest, UpdateRoundRequest, assign_area_round_group,
    create_round, create_round_group, delete_round, delete_round_group, list_round_groups,
    list_rounds, update_round, update_round_group,
};

use super::helpers::{
    create_test_admin, create_test_admin_operator, create_test_bidder, create_test_bidder_operator,
    setup_test_persistence,
};

// ============================================================================
// Round Group Tests
// ============================================================================

#[test]
fn test_create_round_group_success() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    // Get the bid year ID
    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    let request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };

    let result = create_round_group(&mut persistence, bid_year_id, &request, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.round_group_id > 0);
    assert_eq!(response.name, "Regular Round");
    assert!(response.editing_enabled);
}

#[test]
fn test_create_round_group_empty_name_fails() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    let request = CreateRoundGroupRequest {
        name: String::new(),
        editing_enabled: true,
    };

    let result = create_round_group(&mut persistence, bid_year_id, &request, &admin);

    assert!(matches!(result, Err(ApiError::InvalidInput { .. })));
}

#[test]
fn test_create_round_group_whitespace_only_name_fails() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    let request = CreateRoundGroupRequest {
        name: String::from("   "),
        editing_enabled: true,
    };

    let result = create_round_group(&mut persistence, bid_year_id, &request, &admin);

    assert!(matches!(result, Err(ApiError::InvalidInput { .. })));
}

#[test]
fn test_list_round_groups_empty() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    let admin: AuthenticatedActor = create_test_admin();
    let result = list_round_groups(&mut persistence, bid_year_id, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.round_groups.len(), 0);
}

#[test]
fn test_list_round_groups_returns_created_groups() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create first round group
    let request1 = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };
    create_round_group(&mut persistence, bid_year_id, &request1, &admin)
        .expect("Failed to create first round group");

    // Create second round group
    let request2 = CreateRoundGroupRequest {
        name: String::from("Carryover Round"),
        editing_enabled: false,
    };
    create_round_group(&mut persistence, bid_year_id, &request2, &admin)
        .expect("Failed to create second round group");

    // List round groups
    let result = list_round_groups(&mut persistence, bid_year_id, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.round_groups.len(), 2);

    let names: Vec<&str> = response
        .round_groups
        .iter()
        .map(|rg| rg.name.as_str())
        .collect();
    assert!(names.contains(&"Regular Round"));
    assert!(names.contains(&"Carryover Round"));
}

#[test]
fn test_update_round_group_success() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let create_req = CreateRoundGroupRequest {
        name: String::from("Original Name"),
        editing_enabled: true,
    };
    let created = create_round_group(&mut persistence, bid_year_id, &create_req, &admin)
        .expect("Failed to create round group");

    // Update the round group
    let update_req = UpdateRoundGroupRequest {
        round_group_id: created.round_group_id,
        name: String::from("Updated Name"),
        editing_enabled: false,
    };
    let result = update_round_group(&mut persistence, &update_req, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.name, "Updated Name");
    assert!(!response.editing_enabled);
}

#[test]
fn test_delete_round_group_success() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let create_req = CreateRoundGroupRequest {
        name: String::from("To Delete"),
        editing_enabled: true,
    };
    let created = create_round_group(&mut persistence, bid_year_id, &create_req, &admin)
        .expect("Failed to create round group");

    // Delete the round group
    let result = delete_round_group(&mut persistence, created.round_group_id, &admin);

    assert!(result.is_ok());

    // Verify it's deleted by listing
    let list_result = list_round_groups(&mut persistence, bid_year_id, &admin);
    assert!(list_result.is_ok());
    let response = list_result.unwrap();
    assert_eq!(response.round_groups.len(), 0);
}

// ============================================================================
// Round Tests
// ============================================================================

#[test]
fn test_create_round_success() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // First create a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };
    let round_group = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");

    // Create a round
    let request = CreateRoundRequest {
        round_group_id: round_group.round_group_id,
        round_number: 1,
        name: String::from("Round 1"),
        slots_per_day: 10,
        max_groups: 5,
        max_total_hours: 80,
        include_holidays: false,
        allow_overbid: false,
    };

    let result = create_round(&mut persistence, north_area_id, &request, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.round_id > 0);
    assert_eq!(response.round_number, 1);
    assert_eq!(response.name, "Round 1");
    // Note: CreateRoundResponse only includes id, area_id, round_group_id, round_number, name, and message
}

#[test]
fn test_create_round_zero_slots_fails() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };
    let round_group = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");

    // Create a round with zero slots_per_day
    let request = CreateRoundRequest {
        round_group_id: round_group.round_group_id,
        round_number: 1,
        name: String::from("Round 1"),
        slots_per_day: 0, // Invalid
        max_groups: 5,
        max_total_hours: 80,
        include_holidays: false,
        allow_overbid: false,
    };

    let result = create_round(&mut persistence, north_area_id, &request, &admin);

    assert!(matches!(result, Err(ApiError::InvalidInput { .. })));
}

#[test]
fn test_create_round_empty_name_fails() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };
    let round_group = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");

    // Create a round with empty name
    let request = CreateRoundRequest {
        round_group_id: round_group.round_group_id,
        round_number: 1,
        name: String::new(), // Invalid
        slots_per_day: 10,
        max_groups: 5,
        max_total_hours: 80,
        include_holidays: false,
        allow_overbid: false,
    };

    let result = create_round(&mut persistence, north_area_id, &request, &admin);

    assert!(matches!(result, Err(ApiError::InvalidInput { .. })));
}

#[test]
fn test_list_rounds_empty() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");
    let admin: AuthenticatedActor = create_test_admin();

    let result = list_rounds(&mut persistence, north_area_id, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.rounds.len(), 0);
}

#[test]
fn test_list_rounds_returns_created_rounds() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };
    let round_group = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");

    // Create first round
    let request1 = CreateRoundRequest {
        round_group_id: round_group.round_group_id,
        round_number: 1,
        name: String::from("Round 1"),
        slots_per_day: 10,
        max_groups: 5,
        max_total_hours: 80,
        include_holidays: false,
        allow_overbid: false,
    };
    create_round(&mut persistence, north_area_id, &request1, &admin)
        .expect("Failed to create first round");

    // Create second round
    let request2 = CreateRoundRequest {
        round_group_id: round_group.round_group_id,
        round_number: 2,
        name: String::from("Round 2"),
        slots_per_day: 8,
        max_groups: 4,
        max_total_hours: 64,
        include_holidays: true,
        allow_overbid: true,
    };
    create_round(&mut persistence, north_area_id, &request2, &admin)
        .expect("Failed to create second round");

    // List rounds
    let result = list_rounds(&mut persistence, north_area_id, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.rounds.len(), 2);

    let names: Vec<&str> = response.rounds.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Round 1"));
    assert!(names.contains(&"Round 2"));
}

#[test]
fn test_update_round_success() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };
    let round_group = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");

    // Create a round
    let create_req = CreateRoundRequest {
        round_group_id: round_group.round_group_id,
        round_number: 1,
        name: String::from("Original Name"),
        slots_per_day: 10,
        max_groups: 5,
        max_total_hours: 80,
        include_holidays: false,
        allow_overbid: false,
    };
    let created = create_round(&mut persistence, north_area_id, &create_req, &admin)
        .expect("Failed to create round");

    // Update the round
    let update_req = UpdateRoundRequest {
        round_id: created.round_id,
        round_group_id: created.round_group_id,
        round_number: created.round_number,
        name: String::from("Updated Round"),
        slots_per_day: 12,
        max_groups: 6,
        max_total_hours: 96,
        include_holidays: true,
        allow_overbid: true,
    };
    let result = update_round(&mut persistence, &update_req, &admin);

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.name, "Updated Round");
    // Note: UpdateRoundResponse only includes id, area_id, round_group_id, round_number, name, and message
    // To verify the full update, we would need to call list_rounds or get_round
}

#[test]
fn test_delete_round_success() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin: AuthenticatedActor = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };
    let round_group = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");

    // Create a round
    let create_req = CreateRoundRequest {
        round_group_id: round_group.round_group_id,
        round_number: 1,
        name: String::from("To Delete"),
        slots_per_day: 10,
        max_groups: 5,
        max_total_hours: 80,
        include_holidays: false,
        allow_overbid: false,
    };
    let created = create_round(&mut persistence, north_area_id, &create_req, &admin)
        .expect("Failed to create round");

    // Delete the round
    let result = delete_round(&mut persistence, created.round_id, &admin);

    assert!(result.is_ok());

    // Verify it's deleted by listing
    let list_result = list_rounds(&mut persistence, north_area_id, &admin);
    assert!(list_result.is_ok());
    let response = list_result.unwrap();
    assert_eq!(response.rounds.len(), 0);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[test]
fn test_bidder_cannot_create_round_group() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let bidder: AuthenticatedActor = create_test_bidder();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    let request = CreateRoundGroupRequest {
        name: String::from("Regular Round"),
        editing_enabled: true,
    };

    let result = create_round_group(&mut persistence, bid_year_id, &request, &bidder);

    assert!(matches!(result, Err(ApiError::Unauthorized { .. })));
}

#[test]
fn test_bidder_cannot_create_round() {
    use zab_bid_domain::BidYear;

    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let bidder: AuthenticatedActor = create_test_bidder();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Get the area_id for North
    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .expect("Failed to get bid year from ID");
    let areas = persistence
        .list_areas(&BidYear::new(year))
        .expect("Failed to list areas");
    let north_area = areas
        .iter()
        .find(|a| a.area_code() == "NORTH")
        .expect("North area not found");
    let north_area_id = north_area.area_id().expect("Area ID must be set");

    let request = CreateRoundRequest {
        round_group_id: 1, // Doesn't matter, auth will fail first
        round_number: 1,
        name: String::from("Round 1"),
        slots_per_day: 10,
        max_groups: 5,
        max_total_hours: 80,
        include_holidays: false,
        allow_overbid: false,
    };

    let result = create_round(&mut persistence, north_area_id, &request, &bidder);

    assert!(matches!(result, Err(ApiError::Unauthorized { .. })));
}

// ============================================================================
// Area → Round Group Assignment Tests
// ============================================================================

#[test]
fn test_assign_round_group_to_area_success() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Test Round Group"),
        editing_enabled: true,
    };
    let rg_response = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    // Get a non-system area ID (North area from test fixture)
    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");
    let north_area_id = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_code() == "NORTH")
        .and_then(|(_, a)| a.area_id())
        .expect("North area not found");

    let admin_operator = create_test_admin_operator();

    // Assign the round group to the area
    let assign_request = AssignAreaRoundGroupRequest {
        round_group_id: Some(rg_response.round_group_id),
    };

    let result = assign_area_round_group(
        &mut persistence,
        &metadata,
        north_area_id,
        &assign_request,
        &admin,
        &admin_operator,
    );

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.area_id, north_area_id);
    assert_eq!(response.round_group_id, Some(rg_response.round_group_id));

    // Verify persistence
    let assigned_rg_id = persistence
        .get_area_round_group_id(north_area_id)
        .expect("Failed to get round group ID");
    assert_eq!(assigned_rg_id, Some(rg_response.round_group_id));
}

#[test]
fn test_assign_round_group_clear_assignment() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    // Create and assign a round group
    let rg_request = CreateRoundGroupRequest {
        name: String::from("Test Round Group"),
        editing_enabled: true,
    };
    let rg_response = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");
    let north_area_id = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_code() == "NORTH")
        .and_then(|(_, a)| a.area_id())
        .expect("North area not found");

    let admin_operator = create_test_admin_operator();

    let assign_request = AssignAreaRoundGroupRequest {
        round_group_id: Some(rg_response.round_group_id),
    };

    assign_area_round_group(
        &mut persistence,
        &metadata,
        north_area_id,
        &assign_request,
        &admin,
        &admin_operator,
    )
    .expect("Failed to assign round group");

    // Now clear the assignment
    let clear_request = AssignAreaRoundGroupRequest {
        round_group_id: None,
    };

    let result = assign_area_round_group(
        &mut persistence,
        &metadata,
        north_area_id,
        &clear_request,
        &admin,
        &admin_operator,
    );

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.round_group_id, None);

    // Verify persistence
    let assigned_rg_id = persistence
        .get_area_round_group_id(north_area_id)
        .expect("Failed to get round group ID");
    assert_eq!(assigned_rg_id, None);
}

// Note: System area validation test skipped in unit tests since test fixture
// doesn't easily support creating system areas. The validate_not_system_area
// function is tested elsewhere and enforced at the handler level.

#[test]
fn test_assign_round_group_nonexistent_round_group_fails() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin = create_test_admin();

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");
    let north_area_id = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_code() == "NORTH")
        .and_then(|(_, a)| a.area_id())
        .expect("North area not found");

    let admin_operator = create_test_admin_operator();

    let assign_request = AssignAreaRoundGroupRequest {
        round_group_id: Some(999_999), // Non-existent ID
    };

    let result = assign_area_round_group(
        &mut persistence,
        &metadata,
        north_area_id,
        &assign_request,
        &admin,
        &admin_operator,
    );

    assert!(matches!(result, Err(ApiError::ResourceNotFound { .. })));
}

#[test]
fn test_assign_round_group_nonexistent_area_fails() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let admin = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    let rg_request = CreateRoundGroupRequest {
        name: String::from("Test Round Group"),
        editing_enabled: true,
    };
    let rg_response = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let admin_operator = create_test_admin_operator();

    let assign_request = AssignAreaRoundGroupRequest {
        round_group_id: Some(rg_response.round_group_id),
    };

    let result = assign_area_round_group(
        &mut persistence,
        &metadata,
        999_999, // Non-existent area ID
        &assign_request,
        &admin,
        &admin_operator,
    );

    assert!(matches!(result, Err(ApiError::ResourceNotFound { .. })));
}

#[test]
fn test_assign_round_group_bidder_fails() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let bidder = create_test_bidder();
    let admin = create_test_admin();

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Failed to get bid year ID");

    let rg_request = CreateRoundGroupRequest {
        name: String::from("Test Round Group"),
        editing_enabled: true,
    };
    let rg_response = create_round_group(&mut persistence, bid_year_id, &rg_request, &admin)
        .expect("Failed to create round group");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");
    let north_area_id = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_code() == "NORTH")
        .and_then(|(_, a)| a.area_id())
        .expect("North area not found");

    let bidder_operator = create_test_bidder_operator();

    let assign_request = AssignAreaRoundGroupRequest {
        round_group_id: Some(rg_response.round_group_id),
    };

    let result = assign_area_round_group(
        &mut persistence,
        &metadata,
        north_area_id,
        &assign_request,
        &bidder,
        &bidder_operator,
    );

    assert!(matches!(result, Err(ApiError::Unauthorized { .. })));
}
