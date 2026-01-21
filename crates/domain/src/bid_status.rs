// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Bid status tracking and transition logic.
//!
//! This module defines bid status states and valid transitions.
//! Status transitions are operator-initiated only; the system never
//! advances status based on time alone.

use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Bid status states tracking user progress through bidding rounds.
///
/// Status is tracked per user, per round, per area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BidStatus {
    /// User's bid window has not yet begun
    NotStartedPreWindow,
    /// User's bid window is active, but they haven't started bidding
    NotStartedInWindow,
    /// User has started but not completed their bids
    InProgress,
    /// User completed bids before window closed
    CompletedOnTime,
    /// User completed bids after window closed
    CompletedLate,
    /// User did not bid (no call / management pause)
    Missed,
    /// User explicitly opted out of bidding
    VoluntarilyNotBidding,
    /// Bids entered by proxy (on behalf of user)
    Proxy,
}

impl BidStatus {
    /// Returns the string representation of the status.
    ///
    /// This is used for persistence and API serialization.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NotStartedPreWindow => "not_started_pre_window",
            Self::NotStartedInWindow => "not_started_in_window",
            Self::InProgress => "in_progress",
            Self::CompletedOnTime => "completed_on_time",
            Self::CompletedLate => "completed_late",
            Self::Missed => "missed",
            Self::VoluntarilyNotBidding => "voluntarily_not_bidding",
            Self::Proxy => "proxy",
        }
    }

    /// Parses a status from its string representation.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidBidStatus` if the string is not a valid status.
    fn parse_str(s: &str) -> Result<Self, DomainError> {
        match s {
            "not_started_pre_window" => Ok(Self::NotStartedPreWindow),
            "not_started_in_window" => Ok(Self::NotStartedInWindow),
            "in_progress" => Ok(Self::InProgress),
            "completed_on_time" => Ok(Self::CompletedOnTime),
            "completed_late" => Ok(Self::CompletedLate),
            "missed" => Ok(Self::Missed),
            "voluntarily_not_bidding" => Ok(Self::VoluntarilyNotBidding),
            "proxy" => Ok(Self::Proxy),
            _ => Err(DomainError::InvalidBidStatus {
                status: s.to_string(),
            }),
        }
    }

    /// Returns true if this status is terminal (cannot transition to another state).
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::CompletedOnTime
                | Self::CompletedLate
                | Self::Missed
                | Self::VoluntarilyNotBidding
                | Self::Proxy
        )
    }

    /// Validates if a transition from this status to another is permitted.
    ///
    /// # Errors
    ///
    /// Returns an error if the transition is not allowed.
    pub fn validate_transition(&self, new_status: Self) -> Result<(), DomainError> {
        // Cannot transition from terminal states
        if self.is_terminal() {
            return Err(DomainError::InvalidStatusTransition {
                from: self.as_str().to_string(),
                to: new_status.as_str().to_string(),
                reason: "cannot transition from terminal state".to_string(),
            });
        }

        // Valid transitions based on current state
        let valid = match self {
            Self::NotStartedInWindow => matches!(
                new_status,
                Self::InProgress
                    | Self::CompletedOnTime
                    | Self::CompletedLate
                    | Self::Missed
                    | Self::VoluntarilyNotBidding
                    | Self::Proxy
            ),
            Self::InProgress => matches!(new_status, Self::CompletedOnTime | Self::CompletedLate),
            // No operator transitions allowed from pre-window or terminal states
            Self::NotStartedPreWindow
            | Self::CompletedOnTime
            | Self::CompletedLate
            | Self::Missed
            | Self::VoluntarilyNotBidding
            | Self::Proxy => false,
        };

        if valid {
            Ok(())
        } else {
            Err(DomainError::InvalidStatusTransition {
                from: self.as_str().to_string(),
                to: new_status.as_str().to_string(),
                reason: "transition not permitted by status lifecycle rules".to_string(),
            })
        }
    }
}

/// User bid status data including metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBidStatus {
    pub user_id: i64,
    pub round_id: i64,
    pub status: BidStatus,
    pub updated_at: String,
    pub updated_by: i64,
    pub notes: Option<String>,
}

impl FromStr for BidStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_string_round_trip() {
        let statuses = vec![
            BidStatus::NotStartedPreWindow,
            BidStatus::NotStartedInWindow,
            BidStatus::InProgress,
            BidStatus::CompletedOnTime,
            BidStatus::CompletedLate,
            BidStatus::Missed,
            BidStatus::VoluntarilyNotBidding,
            BidStatus::Proxy,
        ];

        for status in statuses {
            let s = status.as_str();
            match BidStatus::parse_str(s) {
                Ok(parsed) => assert_eq!(status, parsed),
                Err(e) => panic!("Failed to parse status string: {s}: {e}"),
            }
        }
    }

    #[test]
    fn test_invalid_status_string() {
        let result = BidStatus::parse_str("invalid_status");
        assert!(result.is_err());
    }

    #[test]
    fn test_terminal_states() {
        assert!(!BidStatus::NotStartedPreWindow.is_terminal());
        assert!(!BidStatus::NotStartedInWindow.is_terminal());
        assert!(!BidStatus::InProgress.is_terminal());
        assert!(BidStatus::CompletedOnTime.is_terminal());
        assert!(BidStatus::CompletedLate.is_terminal());
        assert!(BidStatus::Missed.is_terminal());
        assert!(BidStatus::VoluntarilyNotBidding.is_terminal());
        assert!(BidStatus::Proxy.is_terminal());
    }

    #[test]
    fn test_valid_transitions_from_not_started_in_window() {
        let current = BidStatus::NotStartedInWindow;

        assert!(current.validate_transition(BidStatus::InProgress).is_ok());
        assert!(
            current
                .validate_transition(BidStatus::CompletedOnTime)
                .is_ok()
        );
        assert!(
            current
                .validate_transition(BidStatus::CompletedLate)
                .is_ok()
        );
        assert!(current.validate_transition(BidStatus::Missed).is_ok());
        assert!(
            current
                .validate_transition(BidStatus::VoluntarilyNotBidding)
                .is_ok()
        );
        assert!(current.validate_transition(BidStatus::Proxy).is_ok());
    }

    #[test]
    fn test_valid_transitions_from_in_progress() {
        let current = BidStatus::InProgress;

        assert!(
            current
                .validate_transition(BidStatus::CompletedOnTime)
                .is_ok()
        );
        assert!(
            current
                .validate_transition(BidStatus::CompletedLate)
                .is_ok()
        );
    }

    #[test]
    fn test_invalid_transitions_from_in_progress() {
        let current = BidStatus::InProgress;

        assert!(
            current
                .validate_transition(BidStatus::NotStartedInWindow)
                .is_err()
        );
        assert!(current.validate_transition(BidStatus::Missed).is_err());
        assert!(
            current
                .validate_transition(BidStatus::VoluntarilyNotBidding)
                .is_err()
        );
        assert!(current.validate_transition(BidStatus::Proxy).is_err());
    }

    #[test]
    fn test_no_transitions_from_terminal_states() {
        let terminal_states = vec![
            BidStatus::CompletedOnTime,
            BidStatus::CompletedLate,
            BidStatus::Missed,
            BidStatus::VoluntarilyNotBidding,
            BidStatus::Proxy,
        ];

        for terminal in terminal_states {
            assert!(
                terminal
                    .validate_transition(BidStatus::NotStartedInWindow)
                    .is_err()
            );
            assert!(terminal.validate_transition(BidStatus::InProgress).is_err());
            assert!(
                terminal
                    .validate_transition(BidStatus::CompletedOnTime)
                    .is_err()
            );
        }
    }

    #[test]
    fn test_no_transition_from_pre_window() {
        let current = BidStatus::NotStartedPreWindow;

        // NotStartedPreWindow should only transition via system logic (time-based check),
        // not operator action, so all transitions should fail
        assert!(
            current
                .validate_transition(BidStatus::NotStartedInWindow)
                .is_err()
        );
        assert!(current.validate_transition(BidStatus::InProgress).is_err());
    }
}
