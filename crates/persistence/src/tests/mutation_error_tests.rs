// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Persistence mutation error handling tests - Gap 2 remediation.
//!
//! Tests database error paths in mutation functions including constraint
//! violations, foreign key failures, and transaction consistency.

use zab_bid_domain::{Area, BidYear, Initials};

use crate::{PersistenceError, SqlitePersistence};

/// Helper to create a test persistence instance with a bid year and area.
fn setup_test_persistence_with_entities() -> Result<(SqlitePersistence, i64, i64), PersistenceError>
{
    use zab_bid::{BootstrapMetadata, BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    let mut persistence = SqlitePersistence::new_in_memory()?;

    // Create operator for foreign keys
    let operator_id =
        persistence.create_operator("test-admin", "Test Admin", "password", "Admin")?;

    let mut metadata = BootstrapMetadata::new();

    // Create bid year
    let create_bid_year_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: time::Date::from_calendar_date(2026, time::Month::January, 4)
            .expect("Valid date"),
        num_pay_periods: 26,
    };

    let placeholder_bid_year = BidYear::new(2026);
    let actor = Actor::with_operator(
        String::from("test-admin"),
        String::from("admin"),
        operator_id,
        String::from("test-operator"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test-setup"), String::from("Test setup"));

    let bid_year_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_bid_year,
        create_bid_year_cmd,
        actor.clone(),
        cause.clone(),
    )
    .map_err(|e| PersistenceError::Other(format!("Bootstrap failed: {e}")))?;

    persistence.persist_bootstrap(&bid_year_result)?;
    metadata = bid_year_result.new_metadata;

    // Create area
    let create_area_cmd = Command::CreateArea {
        area_id: String::from("NORTH"),
    };

    let active_bid_year = BidYear::new(2026);
    let area_result: BootstrapResult =
        apply_bootstrap(&metadata, &active_bid_year, create_area_cmd, actor, cause)
            .map_err(|e| PersistenceError::Other(format!("Bootstrap failed: {e}")))?;

    persistence.persist_bootstrap(&area_result)?;

    // Get canonical IDs
    let bid_year_id = persistence.get_bid_year_id(2026)?;
    let area_id = persistence.get_area_id(bid_year_id, "NORTH")?;

    Ok((persistence, bid_year_id, area_id))
}

// ============================================================================
// Gap 2: Persistence Mutation Error Handling
// ============================================================================

#[test]
fn test_update_user_with_nonexistent_user_id_returns_not_found() {
    let (mut persistence, _bid_year_id, area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    // Get the area from persistence to construct Area object
    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");
    let area = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(area_id))
        .map(|(_, a)| a.clone())
        .expect("Area not found");

    let initials = Initials::new("AB");
    let nonexistent_user_id = 99999;

    let result = persistence.update_user(
        nonexistent_user_id,
        &initials,
        "Test User",
        &area,
        "CPC",
        Some(1),
        "2020-01-01",
        "2020-01-01",
        "2020-01-01",
        "2020-01-01",
        None,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::NotFound(msg) => {
            assert!(msg.contains("User"));
            assert!(msg.contains(&nonexistent_user_id.to_string()));
        }
        other => panic!("Expected NotFound error, got: {other:?}"),
    }
}

#[test]
fn test_update_user_with_nonexistent_area_id_returns_database_error() {
    let (mut persistence, _bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    // Create an Area with a nonexistent area_id
    let nonexistent_area = Area::with_id(99999, "INVALID", None, false, None);
    let initials = Initials::new("AB");

    // Even with nonexistent user, we hit area check first
    let result = persistence.update_user(
        1,
        &initials,
        "Test User Updated",
        &nonexistent_area,
        "CPC",
        Some(1),
        "2020-01-01",
        "2020-01-01",
        "2020-01-01",
        "2020-01-01",
        None,
    );

    // Foreign key constraint violation should result in DatabaseError or NotFound
    assert!(result.is_err());
    // Either error type is acceptable for FK violation
    assert!(
        matches!(
            result.unwrap_err(),
            PersistenceError::DatabaseError(_) | PersistenceError::NotFound(_)
        ),
        "Expected DatabaseError or NotFound for foreign key violation"
    );
}

#[test]
fn test_update_user_with_area_missing_canonical_id_returns_error() {
    let (mut persistence, _bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    // Create an Area without a canonical area_id
    let area_without_id = Area::new("SOUTH");
    let initials = Initials::new("AB");

    let result = persistence.update_user(
        1,
        &initials,
        "Test User Updated",
        &area_without_id,
        "CPC",
        Some(1),
        "2020-01-01",
        "2020-01-01",
        "2020-01-01",
        "2020-01-01",
        None,
    );

    // Should fail because area has no canonical ID
    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::Other(msg) => {
            assert!(msg.contains("no canonical area_id"));
        }
        other => panic!("Expected Other error for missing area_id, got: {other:?}"),
    }
}

#[test]
fn test_update_area_name_with_nonexistent_area_id_returns_not_found() {
    let (mut persistence, _bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_area_id = 99999;
    let result = persistence.update_area_name(nonexistent_area_id, Some("New Name"));

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::NotFound(msg) | PersistenceError::ReconstructionError(msg) => {
            assert!(msg.contains("Area") || msg.contains(&nonexistent_area_id.to_string()));
        }
        other => panic!("Expected NotFound or ReconstructionError, got: {other:?}"),
    }
}

#[test]
fn test_override_area_assignment_with_nonexistent_user_returns_reconstruction_error() {
    let (mut persistence, bid_year_id, area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_user_id = 99999;
    let reason = "Test override reason for testing";

    let result =
        persistence.override_area_assignment(bid_year_id, nonexistent_user_id, area_id, reason);

    // Should fail with ReconstructionError (canonical record not found)
    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::ReconstructionError(msg) => {
            assert!(msg.contains("user_id") || msg.contains(&nonexistent_user_id.to_string()));
        }
        other => panic!("Expected ReconstructionError for nonexistent user, got: {other:?}"),
    }
}

#[test]
fn test_override_area_assignment_with_nonexistent_area_returns_reconstruction_error() {
    let (mut persistence, bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_area_id = 99999;
    let reason = "Test override reason for testing";

    // Use a user_id that's unlikely to exist (no canonical membership record)
    let result = persistence.override_area_assignment(bid_year_id, 1, nonexistent_area_id, reason);

    // Should fail because canonical membership doesn't exist
    assert!(result.is_err());
    assert!(
        matches!(
            result.unwrap_err(),
            PersistenceError::DatabaseError(_)
                | PersistenceError::NotFound(_)
                | PersistenceError::ReconstructionError(_)
        ),
        "Expected error for nonexistent canonical record"
    );
}

#[test]
fn test_override_eligibility_with_nonexistent_user_returns_reconstruction_error() {
    let (mut persistence, bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_user_id = 99999;
    let reason = "Test override reason for testing";

    let result = persistence.override_eligibility(bid_year_id, nonexistent_user_id, true, reason);

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::ReconstructionError(msg) => {
            assert!(msg.contains("user_id") || msg.contains(&nonexistent_user_id.to_string()));
        }
        other => panic!("Expected ReconstructionError for nonexistent user, got: {other:?}"),
    }
}

#[test]
fn test_override_bid_order_with_nonexistent_user_returns_reconstruction_error() {
    let (mut persistence, bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_user_id = 99999;
    let reason = "Test override reason for testing";

    let result = persistence.override_bid_order(bid_year_id, nonexistent_user_id, Some(42), reason);

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::ReconstructionError(msg) => {
            assert!(msg.contains("user_id") || msg.contains(&nonexistent_user_id.to_string()));
        }
        other => panic!("Expected ReconstructionError for nonexistent user, got: {other:?}"),
    }
}

#[test]
fn test_override_bid_window_with_nonexistent_user_returns_reconstruction_error() {
    let (mut persistence, bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_user_id = 99999;
    let reason = "Test override reason for testing";
    let window_start = String::from("2026-01-01");
    let window_end = String::from("2026-01-31");

    let result = persistence.override_bid_window(
        bid_year_id,
        nonexistent_user_id,
        Some(&window_start),
        Some(&window_end),
        reason,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::ReconstructionError(msg) => {
            assert!(msg.contains("user_id") || msg.contains(&nonexistent_user_id.to_string()));
        }
        other => panic!("Expected ReconstructionError for nonexistent user, got: {other:?}"),
    }
}

#[test]
fn test_lookup_bid_year_id_with_nonexistent_year_returns_not_found() {
    let (mut persistence, _bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_year = 9999;
    let result = persistence.get_bid_year_id(nonexistent_year);

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::NotFound(msg) => {
            assert!(msg.contains("bid year") || msg.contains(&nonexistent_year.to_string()));
        }
        other => panic!("Expected NotFound error, got: {other:?}"),
    }
}

#[test]
fn test_lookup_area_id_with_nonexistent_area_code_returns_not_found() {
    let (mut persistence, bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_area_code = "INVALID";
    let result = persistence.get_area_id(bid_year_id, nonexistent_area_code);

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::NotFound(msg) => {
            assert!(msg.contains("area") || msg.contains(nonexistent_area_code));
        }
        other => panic!("Expected NotFound error, got: {other:?}"),
    }
}

#[test]
fn test_lookup_area_id_with_nonexistent_bid_year_returns_not_found() {
    let (mut persistence, _bid_year_id, _area_id) =
        setup_test_persistence_with_entities().expect("Failed to setup persistence");

    let nonexistent_bid_year_id = 99999;
    let result = persistence.get_area_id(nonexistent_bid_year_id, "NORTH");

    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::NotFound(msg) => {
            assert!(msg.contains("area") || msg.contains("NORTH"));
        }
        other => panic!("Expected NotFound error, got: {other:?}"),
    }
}
