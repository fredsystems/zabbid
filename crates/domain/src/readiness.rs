// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Phase 29D: Bid Year Readiness Evaluation
//!
//! This module provides logic for evaluating whether a bid year is structurally
//! complete and ready for confirmation to enter bidding.
//!
//! Readiness is **computed**, not stored. It's a pure function of current state.

use crate::bid_order::compute_bid_order;
use crate::types::User;

/// Validates participation flag directional invariant for all users.
///
/// Invariant: `excluded_from_leave_calculation == true` ⇒ `excluded_from_bidding == true`
///
/// # Arguments
///
/// * `users` - All users in the bid year
///
/// # Returns
///
/// Count of users violating the invariant.
#[must_use]
pub fn count_participation_flag_violations(users: &[User]) -> usize {
    users
        .iter()
        .filter(|u| u.excluded_from_leave_calculation && !u.excluded_from_bidding)
        .count()
}

/// Counts users in a system area who have not been reviewed.
///
/// A user is considered reviewed if:
/// - They are not in a system area, OR
/// - They are in a system area AND `no_bid_reviewed` is true
///
/// # Arguments
///
/// * `users` - All users to check
/// * `is_system_area` - Whether the area being checked is a system area
///
/// # Returns
///
/// Count of unreviewed users in system areas.
#[must_use]
pub fn count_unreviewed_no_bid_users(users: &[User], is_system_area: bool) -> usize {
    if !is_system_area {
        return 0;
    }

    users.iter().filter(|u| !u.no_bid_reviewed).count()
}

/// Validates that all users have unique bid order positions.
///
/// Phase 29D: Seniority conflicts are a blocking error. There is no manual resolution path.
///
/// This function attempts to compute the full bid order for the given users.
/// If bid order computation fails due to an unresolved seniority tie, it returns 1.
/// Otherwise, it returns 0.
///
/// # Arguments
///
/// * `users` - All users in the area (excluded users are filtered during computation)
///
/// # Returns
///
/// Number of conflicts detected:
/// - `0` if bid order can be computed without conflicts
/// - `1` if a seniority conflict exists
///
/// # Note
///
/// This function returns a count for consistency with other readiness functions,
/// but the actual conflict is binary (exists or doesn't exist).
#[must_use]
pub fn count_seniority_conflicts(users: &[User]) -> usize {
    // Attempt to compute bid order
    // If computation succeeds, there are no conflicts
    // If computation fails, there is a conflict
    match compute_bid_order(users) {
        Ok(_) => 0,
        Err(_) => 1, // Conflict detected
    }
}

/// Evaluates readiness for a single area.
///
/// # Arguments
///
/// * `area_code` - The area code (for reporting)
/// * `users` - Users in this area
/// * `is_system_area` - Whether this is a system area
/// * `has_rounds` - Whether rounds are configured for this area
///
/// # Returns
///
/// Tuple of (`blocking_reasons`, `unreviewed_count`, `violation_count`)
#[must_use]
pub fn evaluate_area_readiness(
    area_code: &str,
    users: &[User],
    is_system_area: bool,
    has_rounds: bool,
) -> (Vec<String>, usize, usize) {
    let mut blocking_reasons = Vec::new();

    // System areas don't require rounds
    if !is_system_area && !has_rounds {
        blocking_reasons.push(format!("Area '{area_code}' has no rounds configured"));
    }

    let unreviewed_count = count_unreviewed_no_bid_users(users, is_system_area);
    if unreviewed_count > 0 {
        blocking_reasons.push(format!(
            "{unreviewed_count} users in No Bid area have not been reviewed"
        ));
    }

    let violation_count = count_participation_flag_violations(users);
    if violation_count > 0 {
        blocking_reasons.push(format!(
            "{violation_count} users violate participation flag invariant"
        ));
    }

    (blocking_reasons, unreviewed_count, violation_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Area, BidYear, Crew, Initials, SeniorityData, UserType};

    fn create_test_seniority_data() -> SeniorityData {
        SeniorityData::new(
            String::from("2020-01-01"),
            String::from("2020-01-01"),
            String::from("2020-01-01"),
            String::from("2020-01-01"),
            None,
        )
    }

    fn create_test_user(
        excluded_from_bidding: bool,
        excluded_from_leave: bool,
        no_bid_reviewed: bool,
    ) -> User {
        User::new(
            BidYear::new(2026),
            Initials::new("AB"),
            String::from("Test User"),
            Area::new("North"),
            UserType::CPC,
            Crew::new(1).ok(),
            create_test_seniority_data(),
            excluded_from_bidding,
            excluded_from_leave,
            no_bid_reviewed,
        )
    }

    #[test]
    fn test_participation_flag_violations_none() {
        let users = vec![
            create_test_user(false, false, false),
            create_test_user(true, true, false),
            create_test_user(true, false, false),
        ];

        assert_eq!(count_participation_flag_violations(&users), 0);
    }

    #[test]
    fn test_participation_flag_violations_detected() {
        let users = vec![
            create_test_user(false, false, false),
            create_test_user(false, true, false), // Violation!
            create_test_user(true, true, false),
        ];

        assert_eq!(count_participation_flag_violations(&users), 1);
    }

    #[test]
    fn test_unreviewed_no_bid_users_not_system_area() {
        let users = vec![
            create_test_user(false, false, false),
            create_test_user(false, false, false),
        ];

        assert_eq!(count_unreviewed_no_bid_users(&users, false), 0);
    }

    #[test]
    fn test_unreviewed_no_bid_users_system_area() {
        let users = vec![
            create_test_user(false, false, false), // Not reviewed
            create_test_user(false, false, true),  // Reviewed
            create_test_user(false, false, false), // Not reviewed
        ];

        assert_eq!(count_unreviewed_no_bid_users(&users, true), 2);
    }

    #[test]
    fn test_evaluate_area_readiness_all_good() {
        let users = vec![
            create_test_user(false, false, true),
            create_test_user(true, true, true),
        ];

        let (reasons, unreviewed, violations) =
            evaluate_area_readiness("North", &users, false, true);

        assert_eq!(reasons.len(), 0);
        assert_eq!(unreviewed, 0);
        assert_eq!(violations, 0);
    }

    #[test]
    fn test_evaluate_area_readiness_missing_rounds() {
        let users = vec![create_test_user(false, false, true)];

        let (reasons, _, _) = evaluate_area_readiness("North", &users, false, false);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("no rounds configured"));
    }

    #[test]
    fn test_evaluate_area_readiness_system_area_no_rounds_ok() {
        let users = vec![create_test_user(false, false, true)];

        let (reasons, _, _) = evaluate_area_readiness("No Bid", &users, true, false);

        // System areas don't require rounds
        assert_eq!(reasons.len(), 0);
    }

    #[test]
    fn test_evaluate_area_readiness_unreviewed_users() {
        let users = vec![
            create_test_user(false, false, false), // Not reviewed
            create_test_user(false, false, true),  // Reviewed
        ];

        let (reasons, unreviewed, _) = evaluate_area_readiness("No Bid", &users, true, false);

        assert_eq!(unreviewed, 1);
        assert!(reasons.iter().any(|r| r.contains("not been reviewed")));
    }

    #[test]
    fn test_evaluate_area_readiness_participation_violations() {
        let users = vec![
            create_test_user(false, true, true), // Violation!
        ];

        let (reasons, _, violations) = evaluate_area_readiness("North", &users, false, true);

        assert_eq!(violations, 1);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("participation flag invariant"))
        );
    }
}
