// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Capability computation for authorization-aware UI gating.
//!
//! Capabilities expose what actions an operator is permitted to perform
//! without leaking domain internals. They are advisory only and do not
//! replace backend authorization checks.

use crate::auth::{AuthenticatedActor, Role};
use crate::request_response::{
    Capability, GlobalCapabilities, OperatorCapabilities, UserCapabilities,
};
use zab_bid_persistence::{OperatorData, SqlitePersistence};

/// Computes global capabilities for an authenticated operator.
///
/// Global capabilities depend on:
/// - Operator role
/// - Operator disabled state
/// - System-wide state (e.g., bootstrap complete)
///
/// # Arguments
///
/// * `actor` - The authenticated actor
/// * `operator` - The operator data
///
/// # Returns
///
/// A `GlobalCapabilities` struct with all capability flags set.
///
/// # Errors
///
/// Returns an error if database queries fail.
pub const fn compute_global_capabilities(
    actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<GlobalCapabilities, &'static str> {
    // Disabled operators have no capabilities
    if operator.is_disabled {
        return Ok(GlobalCapabilities {
            can_create_operator: Capability::Denied,
            can_create_bid_year: Capability::Denied,
            can_create_area: Capability::Denied,
            can_create_user: Capability::Denied,
            can_modify_users: Capability::Denied,
            can_bootstrap: Capability::Denied,
        });
    }

    // Role-based capabilities
    match actor.role {
        Role::Admin => Ok(GlobalCapabilities {
            can_create_operator: Capability::Allowed,
            can_create_bid_year: Capability::Allowed,
            can_create_area: Capability::Allowed,
            can_create_user: Capability::Allowed,
            can_modify_users: Capability::Allowed,
            can_bootstrap: Capability::Allowed,
        }),
        Role::Bidder => Ok(GlobalCapabilities {
            can_create_operator: Capability::Denied,
            can_create_bid_year: Capability::Denied,
            can_create_area: Capability::Denied,
            can_create_user: Capability::Denied,
            can_modify_users: Capability::Allowed, // Bidders can modify user data (crew assignments, etc.)
            can_bootstrap: Capability::Denied,
        }),
    }
}

/// Computes target-specific capabilities for an operator instance.
///
/// Target-specific capabilities depend on:
/// - The authenticated actor's role and state
/// - The target operator's role and state
/// - Domain invariants (e.g., "last active admin")
///
/// # Arguments
///
/// * `actor` - The authenticated actor
/// * `actor_operator` - The authenticated operator's data
/// * `target_operator` - The target operator being evaluated
/// * `persistence` - The persistence layer (for checking invariants)
///
/// # Returns
///
/// An `OperatorCapabilities` struct with capability flags for this operator.
///
/// # Errors
///
/// Returns an error if database queries fail.
pub fn compute_operator_capabilities(
    actor: &AuthenticatedActor,
    actor_operator: &OperatorData,
    target_operator: &OperatorData,
    persistence: &mut SqlitePersistence,
) -> Result<OperatorCapabilities, String> {
    // Disabled actors have no capabilities
    if actor_operator.is_disabled {
        return Ok(OperatorCapabilities {
            can_disable: Capability::Denied,
            can_delete: Capability::Denied,
        });
    }

    // Only admins can disable or delete operators
    if actor.role != Role::Admin {
        return Ok(OperatorCapabilities {
            can_disable: Capability::Denied,
            can_delete: Capability::Denied,
        });
    }

    // Check if this is the last active admin
    let is_last_active_admin: bool =
        if target_operator.role == "Admin" && !target_operator.is_disabled {
            let active_admin_count: i64 = persistence
                .count_active_admin_operators()
                .map_err(|e| format!("Failed to count active admins: {e}"))?;
            active_admin_count <= 1
        } else {
            false
        };

    // Cannot disable or delete the last active admin
    let can_disable = Capability::from_bool(!is_last_active_admin);
    let can_delete = Capability::from_bool(!is_last_active_admin);

    Ok(OperatorCapabilities {
        can_disable,
        can_delete,
    })
}

/// Computes target-specific capabilities for a user instance.
///
/// Target-specific capabilities depend on:
/// - The authenticated actor's role and state
/// - The target user's state (e.g., whether they have bids)
/// - Domain invariants (e.g., bidding locks)
///
/// # Arguments
///
/// * `actor` - The authenticated actor
/// * `actor_operator` - The authenticated operator's data
///
/// # Returns
///
/// A `UserCapabilities` struct with capability flags for this user.
///
/// # Errors
///
/// Returns an error if database queries fail.
pub const fn compute_user_capabilities(
    actor: &AuthenticatedActor,
    actor_operator: &OperatorData,
) -> Result<UserCapabilities, &'static str> {
    // Disabled actors have no capabilities
    if actor_operator.is_disabled {
        return Ok(UserCapabilities {
            can_delete: Capability::Denied,
            can_move_area: Capability::Denied,
            can_edit_seniority: Capability::Denied,
        });
    }

    // Only admins can delete users or move them between areas
    // Bidders can edit seniority data
    match actor.role {
        Role::Admin => Ok(UserCapabilities {
            can_delete: Capability::Allowed, // TODO: Phase 26 will restrict based on bid data
            can_move_area: Capability::Allowed, // TODO: Future phase will restrict based on bidding state
            can_edit_seniority: Capability::Allowed,
        }),
        Role::Bidder => Ok(UserCapabilities {
            can_delete: Capability::Denied,
            can_move_area: Capability::Denied,
            can_edit_seniority: Capability::Allowed, // Bidders can edit seniority
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_admin() -> AuthenticatedActor {
        AuthenticatedActor::new(String::from("test_admin"), Role::Admin)
    }

    fn create_test_bidder() -> AuthenticatedActor {
        AuthenticatedActor::new(String::from("test_bidder"), Role::Bidder)
    }

    fn create_operator_data(
        operator_id: i64,
        login_name: &str,
        role: &str,
        is_disabled: bool,
    ) -> OperatorData {
        OperatorData {
            operator_id,
            login_name: String::from(login_name),
            display_name: String::from("Test Operator"),
            password_hash: String::from("hash"),
            role: String::from(role),
            is_disabled,
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Iso8601::DEFAULT)
                .unwrap(),
            disabled_at: None,
            last_login_at: None,
        }
    }

    #[test]
    fn test_global_capabilities_admin_active() {
        let actor = create_test_admin();
        let operator = create_operator_data(1, "admin", "Admin", false);

        let caps = compute_global_capabilities(&actor, &operator).unwrap();

        assert!(caps.can_create_operator.is_allowed());
        assert!(caps.can_create_bid_year.is_allowed());
        assert!(caps.can_create_area.is_allowed());
        assert!(caps.can_create_user.is_allowed());
        assert!(caps.can_modify_users.is_allowed());
        assert!(caps.can_bootstrap.is_allowed());
    }

    #[test]
    fn test_global_capabilities_admin_disabled() {
        let actor = create_test_admin();
        let operator = create_operator_data(1, "admin", "Admin", true);

        let caps = compute_global_capabilities(&actor, &operator).unwrap();

        assert!(!caps.can_create_operator.is_allowed());
        assert!(!caps.can_create_bid_year.is_allowed());
        assert!(!caps.can_create_area.is_allowed());
        assert!(!caps.can_create_user.is_allowed());
        assert!(!caps.can_modify_users.is_allowed());
        assert!(!caps.can_bootstrap.is_allowed());
    }

    #[test]
    fn test_global_capabilities_bidder_active() {
        let actor = create_test_bidder();
        let operator = create_operator_data(1, "bidder", "Bidder", false);

        let caps = compute_global_capabilities(&actor, &operator).unwrap();

        assert!(!caps.can_create_operator.is_allowed());
        assert!(!caps.can_create_bid_year.is_allowed());
        assert!(!caps.can_create_area.is_allowed());
        assert!(!caps.can_create_user.is_allowed());
        assert!(caps.can_modify_users.is_allowed()); // Bidders can modify users
        assert!(!caps.can_bootstrap.is_allowed());
    }

    #[test]
    fn test_global_capabilities_bidder_disabled() {
        let actor = create_test_bidder();
        let operator = create_operator_data(1, "bidder", "Bidder", true);

        let caps = compute_global_capabilities(&actor, &operator).unwrap();

        assert!(!caps.can_create_operator.is_allowed());
        assert!(!caps.can_create_bid_year.is_allowed());
        assert!(!caps.can_create_area.is_allowed());
        assert!(!caps.can_create_user.is_allowed());
        assert!(!caps.can_modify_users.is_allowed());
        assert!(!caps.can_bootstrap.is_allowed());
    }

    #[test]
    fn test_operator_capabilities_last_active_admin_cannot_be_disabled() {
        let mut persistence = SqlitePersistence::new_in_memory().unwrap();
        let actor = create_test_admin();

        // Create a single admin operator
        let admin_id = persistence
            .create_operator("admin1", "Admin One", "password", "Admin")
            .unwrap();
        let admin_operator = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
        let actor_operator = admin_operator.clone();

        let caps = compute_operator_capabilities(
            &actor,
            &actor_operator,
            &admin_operator,
            &mut persistence,
        )
        .unwrap();

        assert!(!caps.can_disable.is_allowed());
        assert!(!caps.can_delete.is_allowed());
    }

    #[test]
    fn test_operator_capabilities_can_disable_when_multiple_admins() {
        let mut persistence = SqlitePersistence::new_in_memory().unwrap();
        let actor = create_test_admin();

        // Create two admin operators
        let admin1_id = persistence
            .create_operator("admin1", "Admin One", "password", "Admin")
            .unwrap();
        let admin1_operator = persistence.get_operator_by_id(admin1_id).unwrap().unwrap();

        let admin2_id = persistence
            .create_operator("admin2", "Admin Two", "password", "Admin")
            .unwrap();
        let admin2_operator = persistence.get_operator_by_id(admin2_id).unwrap().unwrap();

        let caps = compute_operator_capabilities(
            &actor,
            &admin1_operator,
            &admin2_operator,
            &mut persistence,
        )
        .unwrap();

        assert!(caps.can_disable.is_allowed());
        assert!(caps.can_delete.is_allowed());
    }

    #[test]
    fn test_operator_capabilities_disabled_admin_can_be_deleted() {
        let mut persistence = SqlitePersistence::new_in_memory().unwrap();
        let actor = create_test_admin();

        // Create two admins, disable one
        let admin1_id = persistence
            .create_operator("admin1", "Admin One", "password", "Admin")
            .unwrap();
        let admin1_operator = persistence.get_operator_by_id(admin1_id).unwrap().unwrap();

        let admin2_id = persistence
            .create_operator("admin2", "Admin Two", "password", "Admin")
            .unwrap();
        persistence.disable_operator(admin2_id).unwrap();
        let admin2_operator = persistence.get_operator_by_id(admin2_id).unwrap().unwrap();

        // Disabled admin can be deleted
        let caps = compute_operator_capabilities(
            &actor,
            &admin1_operator,
            &admin2_operator,
            &mut persistence,
        )
        .unwrap();

        assert!(caps.can_delete.is_allowed());
        assert!(caps.can_disable.is_allowed()); // Can disable an already-disabled operator
    }

    #[test]
    fn test_operator_capabilities_bidder_cannot_disable() {
        let mut persistence = SqlitePersistence::new_in_memory().unwrap();
        let actor = create_test_bidder();

        let bidder_id = persistence
            .create_operator("bidder1", "Bidder One", "password", "Bidder")
            .unwrap();
        let bidder_operator = persistence.get_operator_by_id(bidder_id).unwrap().unwrap();

        let admin_id = persistence
            .create_operator("admin1", "Admin One", "password", "Admin")
            .unwrap();
        let admin_operator = persistence.get_operator_by_id(admin_id).unwrap().unwrap();

        let caps = compute_operator_capabilities(
            &actor,
            &bidder_operator,
            &admin_operator,
            &mut persistence,
        )
        .unwrap();

        assert!(!caps.can_disable.is_allowed());
        assert!(!caps.can_delete.is_allowed());
    }

    #[test]
    fn test_operator_capabilities_disabled_actor_has_no_capabilities() {
        let mut persistence = SqlitePersistence::new_in_memory().unwrap();
        let actor = create_test_admin();

        let admin1_id = persistence
            .create_operator("admin1", "Admin One", "password", "Admin")
            .unwrap();
        persistence.disable_operator(admin1_id).unwrap();
        let admin1_operator = persistence.get_operator_by_id(admin1_id).unwrap().unwrap();

        let admin2_id = persistence
            .create_operator("admin2", "Admin Two", "password", "Admin")
            .unwrap();
        let admin2_operator = persistence.get_operator_by_id(admin2_id).unwrap().unwrap();

        let caps = compute_operator_capabilities(
            &actor,
            &admin1_operator,
            &admin2_operator,
            &mut persistence,
        )
        .unwrap();

        assert!(!caps.can_disable.is_allowed());
        assert!(!caps.can_delete.is_allowed());
    }

    #[test]
    fn test_user_capabilities_admin() {
        let actor = create_test_admin();
        let operator = create_operator_data(1, "admin", "Admin", false);

        let caps = compute_user_capabilities(&actor, &operator).unwrap();

        assert!(caps.can_delete.is_allowed());
        assert!(caps.can_move_area.is_allowed());
        assert!(caps.can_edit_seniority.is_allowed());
    }

    #[test]
    fn test_user_capabilities_bidder() {
        let actor = create_test_bidder();
        let operator = create_operator_data(1, "bidder", "Bidder", false);

        let caps = compute_user_capabilities(&actor, &operator).unwrap();

        assert!(!caps.can_delete.is_allowed());
        assert!(!caps.can_move_area.is_allowed());
        assert!(caps.can_edit_seniority.is_allowed()); // Bidders can edit seniority
    }

    #[test]
    fn test_user_capabilities_disabled_actor() {
        let actor = create_test_admin();
        let operator = create_operator_data(1, "admin", "Admin", true);

        let caps = compute_user_capabilities(&actor, &operator).unwrap();

        assert!(!caps.can_delete.is_allowed());
        assert!(!caps.can_move_area.is_allowed());
        assert!(!caps.can_edit_seniority.is_allowed());
    }
}
