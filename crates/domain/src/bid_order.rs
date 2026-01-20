// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Bid order computation based on strict seniority rules.
//!
//! This module provides deterministic bid order computation for users within an area.
//! Bid order is determined by a strict total ordering on seniority criteria.
//!
//! ## Seniority Ordering Rules (Authoritative)
//!
//! Users are ordered by:
//! 1. Cumulative NATCA Bargaining Unit Time (earliest wins)
//! 2. Tie Breaker 1: NATCA Bargaining Unit Time (earliest wins)
//! 3. Tie Breaker 2: EOD/FAA Date (earliest wins)
//! 4. Tie Breaker 3: Service Computation Date (earliest wins)
//! 5. Tie Breaker 4: Lottery value (lowest wins)
//!
//! ## Invariants
//!
//! - There must NEVER be a tie after applying all rules
//! - Any unresolved tie is a domain violation
//! - System areas are excluded from ordering
//! - Users with `excluded_from_bidding = true` are excluded
//!
//! ## Usage
//!
//! This logic is used by:
//! - Readiness evaluation (to detect conflicts)
//! - Derived bid order preview API (pre-confirmation)
//! - Bid order freezing (at confirmation)

use crate::error::DomainError;
use crate::types::User;

/// Represents a user's position in the derived bid order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidOrderPosition {
    /// The user's canonical ID.
    pub user_id: i64,
    /// The user's initials (for display).
    pub initials: String,
    /// The 1-based position in the bid order (1 = first to bid).
    pub position: usize,
    /// Seniority inputs used for ordering (for transparency).
    pub seniority_inputs: SeniorityInputs,
}

/// Seniority inputs used for bid order computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeniorityInputs {
    /// Cumulative NATCA bargaining unit date.
    pub cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date.
    pub natca_bu_date: String,
    /// Entry on Duty / FAA date.
    pub eod_faa_date: String,
    /// Service Computation Date.
    pub service_computation_date: String,
    /// Lottery value (deterministic tie-breaker).
    pub lottery_value: Option<u32>,
}

impl SeniorityInputs {
    /// Creates seniority inputs from a User.
    #[must_use]
    pub fn from_user(user: &User) -> Self {
        Self {
            cumulative_natca_bu_date: user.seniority_data.cumulative_natca_bu_date.clone(),
            natca_bu_date: user.seniority_data.natca_bu_date.clone(),
            eod_faa_date: user.seniority_data.eod_faa_date.clone(),
            service_computation_date: user.seniority_data.service_computation_date.clone(),
            lottery_value: user.seniority_data.lottery_value,
        }
    }
}

/// Computes the derived bid order for a set of users.
///
/// # Arguments
///
/// * `users` - Users to order (must all be from the same area)
///
/// # Returns
///
/// A vector of `BidOrderPosition` in order (position 1 is first).
///
/// # Errors
///
/// Returns an error if:
/// - A seniority tie cannot be resolved (domain violation)
/// - Seniority data is malformed
///
/// # Seniority Ordering
///
/// Users are ordered by:
/// 1. Cumulative NATCA BU date (earliest first)
/// 2. NATCA BU date (earliest first)
/// 3. EOD/FAA date (earliest first)
/// 4. SCD (earliest first)
/// 5. Lottery value (lowest first)
///
/// All ties MUST be resolved. An unresolved tie is a domain error.
pub fn compute_bid_order(users: &[User]) -> Result<Vec<BidOrderPosition>, DomainError> {
    // Filter out users excluded from bidding
    let eligible_users: Vec<&User> = users.iter().filter(|u| !u.excluded_from_bidding).collect();

    if eligible_users.is_empty() {
        return Ok(Vec::new());
    }

    // Sort users by seniority rules
    let mut sorted_users: Vec<&User> = eligible_users.clone();
    sorted_users.sort_by(|a, b| compare_seniority(a, b));

    // Validate that there are no ties
    for i in 0..sorted_users.len().saturating_sub(1) {
        let current = sorted_users[i];
        let next = sorted_users[i + 1];

        if compare_seniority(current, next) == std::cmp::Ordering::Equal {
            return Err(DomainError::SeniorityConflict {
                user1_initials: current.initials.value().to_string(),
                user2_initials: next.initials.value().to_string(),
                reason: String::from("Unresolved seniority tie after applying all ordering rules"),
            });
        }
    }

    // Build ordered position list
    let positions: Vec<BidOrderPosition> = sorted_users
        .iter()
        .enumerate()
        .filter_map(|(index, user)| {
            user.user_id.map(|uid| BidOrderPosition {
                user_id: uid,
                initials: user.initials.value().to_string(),
                position: index + 1, // 1-based position
                seniority_inputs: SeniorityInputs::from_user(user),
            })
        })
        .collect();

    Ok(positions)
}

/// Compares two users by seniority rules.
///
/// Returns:
/// - `Ordering::Less` if `a` has higher seniority (should bid first)
/// - `Ordering::Greater` if `b` has higher seniority
/// - `Ordering::Equal` if tie (should not happen after all rules)
fn compare_seniority(a: &User, b: &User) -> std::cmp::Ordering {
    // 1. Cumulative NATCA BU Date (earliest wins)
    match a
        .seniority_data
        .cumulative_natca_bu_date
        .cmp(&b.seniority_data.cumulative_natca_bu_date)
    {
        std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
        std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
        std::cmp::Ordering::Equal => {}
    }

    // 2. NATCA BU Date (earliest wins)
    match a
        .seniority_data
        .natca_bu_date
        .cmp(&b.seniority_data.natca_bu_date)
    {
        std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
        std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
        std::cmp::Ordering::Equal => {}
    }

    // 3. EOD/FAA Date (earliest wins)
    match a
        .seniority_data
        .eod_faa_date
        .cmp(&b.seniority_data.eod_faa_date)
    {
        std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
        std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
        std::cmp::Ordering::Equal => {}
    }

    // 4. Service Computation Date (earliest wins)
    match a
        .seniority_data
        .service_computation_date
        .cmp(&b.seniority_data.service_computation_date)
    {
        std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
        std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
        std::cmp::Ordering::Equal => {}
    }

    // 5. Lottery value (lowest wins)
    // Both must have lottery values for a valid comparison
    match (
        a.seniority_data.lottery_value,
        b.seniority_data.lottery_value,
    ) {
        (Some(lottery_a), Some(lottery_b)) => lottery_a.cmp(&lottery_b),
        (Some(_), None) => std::cmp::Ordering::Less, // a has lottery, b doesn't
        (None, Some(_)) => std::cmp::Ordering::Greater, // b has lottery, a doesn't
        (None, None) => std::cmp::Ordering::Equal,   // Both missing lottery - tie
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Initials, SeniorityData, UserType};

    #[allow(clippy::too_many_arguments)]
    fn create_test_user(
        user_id: i64,
        initials: &str,
        cumulative: &str,
        natca_bu: &str,
        eod: &str,
        scd: &str,
        lottery: Option<u32>,
        excluded_from_bidding: bool,
    ) -> User {
        use crate::types::{Area, BidYear};

        User::with_id(
            user_id,
            BidYear::new(2026),
            Initials::new(initials),
            format!("User {initials}"),
            Area::new("Test"),
            UserType::CPC,
            None, // crew
            SeniorityData::new(
                cumulative.to_string(),
                natca_bu.to_string(),
                eod.to_string(),
                scd.to_string(),
                lottery,
            ),
            excluded_from_bidding,
            false, // excluded_from_leave_calculation
            false, // no_bid_reviewed
        )
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_single_user_no_conflict() {
        let users = vec![create_test_user(
            1,
            "ABC",
            "2020-01-01",
            "2020-01-01",
            "2020-01-01",
            "2020-01-01",
            Some(1),
            false,
        )];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].position, 1);
        assert_eq!(result[0].user_id, 1);
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_order_by_cumulative_natca_bu_date() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-06-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(1),
                false,
            ),
            create_test_user(
                2,
                "DEF",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(2),
                false,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].user_id, 2); // Earlier cumulative date
        assert_eq!(result[0].position, 1);
        assert_eq!(result[1].user_id, 1);
        assert_eq!(result[1].position, 2);
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_tie_broken_by_natca_bu_date() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-01-01",
                "2020-06-01",
                "2020-01-01",
                "2020-01-01",
                Some(1),
                false,
            ),
            create_test_user(
                2,
                "DEF",
                "2020-01-01",
                "2020-03-01",
                "2020-01-01",
                "2020-01-01",
                Some(2),
                false,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].user_id, 2); // Earlier NATCA BU date
        assert_eq!(result[1].user_id, 1);
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_tie_broken_by_eod_date() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-01-01",
                "2020-01-01",
                "2020-06-01",
                "2020-01-01",
                Some(1),
                false,
            ),
            create_test_user(
                2,
                "DEF",
                "2020-01-01",
                "2020-01-01",
                "2020-03-01",
                "2020-01-01",
                Some(2),
                false,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].user_id, 2); // Earlier EOD date
        assert_eq!(result[1].user_id, 1);
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_tie_broken_by_scd() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-06-01",
                Some(1),
                false,
            ),
            create_test_user(
                2,
                "DEF",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-03-01",
                Some(2),
                false,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].user_id, 2); // Earlier SCD
        assert_eq!(result[1].user_id, 1);
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_tie_broken_by_lottery() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(5),
                false,
            ),
            create_test_user(
                2,
                "DEF",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(2),
                false,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].user_id, 2); // Lower lottery value
        assert_eq!(result[1].user_id, 1);
    }

    #[test]
    fn test_unresolved_tie_returns_error() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                None,
                false,
            ),
            create_test_user(
                2,
                "DEF",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                None,
                false,
            ),
        ];

        let result = compute_bid_order(&users);
        assert!(result.is_err());

        if let Err(DomainError::SeniorityConflict {
            user1_initials,
            user2_initials,
            ..
        }) = result
        {
            assert!(user1_initials == "ABC" || user1_initials == "DEF");
            assert!(user2_initials == "ABC" || user2_initials == "DEF");
            assert_ne!(user1_initials, user2_initials);
        } else {
            panic!("Expected SeniorityConflict error");
        }
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_excluded_users_are_filtered() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(1),
                true, // excluded
            ),
            create_test_user(
                2,
                "DEF",
                "2020-06-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(2),
                false,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].user_id, 2); // Only non-excluded user
        assert_eq!(result[0].position, 1);
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_all_excluded_returns_empty() {
        let users = vec![
            create_test_user(
                1,
                "ABC",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(1),
                true,
            ),
            create_test_user(
                2,
                "DEF",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(2),
                true,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert!(result.is_empty());
    }

    #[allow(clippy::expect_used)]
    #[test]
    fn test_complex_ordering() {
        let users = vec![
            create_test_user(
                1,
                "AAA",
                "2018-01-01",
                "2018-01-01",
                "2018-01-01",
                "2018-01-01",
                Some(3),
                false,
            ),
            create_test_user(
                2,
                "BBB",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                "2020-01-01",
                Some(1),
                false,
            ),
            create_test_user(
                3,
                "CCC",
                "2018-01-01",
                "2018-06-01",
                "2018-01-01",
                "2018-01-01",
                Some(2),
                false,
            ),
            create_test_user(
                4,
                "DDD",
                "2018-01-01",
                "2018-01-01",
                "2018-01-01",
                "2018-01-01",
                Some(1),
                false,
            ),
        ];

        let result = compute_bid_order(&users).expect("should succeed");
        assert_eq!(result.len(), 4);

        // Expected order:
        // 1. DDD (cumulative 2018, natca 2018, eod 2018, scd 2018, lottery 1)
        // 2. AAA (cumulative 2018, natca 2018, eod 2018, scd 2018, lottery 3)
        // 3. CCC (cumulative 2018, natca 2018-06, later NATCA BU)
        // 4. BBB (cumulative 2020, later cumulative)
        assert_eq!(result[0].user_id, 4); // DDD
        assert_eq!(result[1].user_id, 1); // AAA
        assert_eq!(result[2].user_id, 3); // CCC
        assert_eq!(result[3].user_id, 2); // BBB
    }
}
