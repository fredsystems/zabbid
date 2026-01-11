// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use zab_bid_audit::{AuditEvent, StateSnapshot};
use zab_bid_domain::{Area, BidYear, CanonicalBidYear, User};

/// Bootstrap metadata tracking which bid years and areas exist.
///
/// This is separate from the scoped State and represents global system metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapMetadata {
    /// All valid bid years that have been created.
    pub bid_years: Vec<BidYear>,
    /// All valid areas per bid year.
    pub areas: Vec<(BidYear, Area)>,
}

impl BootstrapMetadata {
    /// Creates a new empty bootstrap metadata.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bid_years: Vec::new(),
            areas: Vec::new(),
        }
    }

    /// Checks if a bid year exists.
    #[must_use]
    pub fn has_bid_year(&self, bid_year: &BidYear) -> bool {
        self.bid_years.contains(bid_year)
    }

    /// Checks if an area exists in a bid year.
    #[must_use]
    pub fn has_area(&self, bid_year: &BidYear, area: &Area) -> bool {
        self.areas.iter().any(|(y, a)| y == bid_year && a == area)
    }

    /// Adds a bid year.
    pub(crate) fn add_bid_year(&mut self, bid_year: BidYear) {
        self.bid_years.push(bid_year);
    }

    /// Adds an area to a bid year.
    pub(crate) fn add_area(&mut self, bid_year: BidYear, area: Area) {
        self.areas.push((bid_year, area));
    }
}

impl Default for BootstrapMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// The complete system state scoped to a single `(bid_year, area)` pair.
///
/// State is now scoped to one bid year and one area combination.
/// This enables proper persistence and audit scoping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// The bid year this state is scoped to.
    pub bid_year: BidYear,
    /// The area this state is scoped to.
    pub area: Area,
    /// All registered users for this `(bid_year, area)`.
    pub users: Vec<User>,
}

impl State {
    /// Creates a new empty state for a given bid year and area.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year this state is scoped to
    /// * `area` - The area this state is scoped to
    #[must_use]
    pub const fn new(bid_year: BidYear, area: Area) -> Self {
        Self {
            bid_year,
            area,
            users: Vec::new(),
        }
    }

    /// Converts the state to a snapshot for audit purposes.
    #[must_use]
    pub fn to_snapshot(&self) -> StateSnapshot {
        StateSnapshot::new(format!(
            "bid_year={},area={},users_count={}",
            self.bid_year.year(),
            self.area.id(),
            self.users.len()
        ))
    }
}

/// The result of a successful state transition.
///
/// Transitions are atomic: they either succeed completely or fail without side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionResult {
    /// The new state after the transition.
    pub new_state: State,
    /// The audit event recording this transition.
    pub audit_event: AuditEvent,
}

/// The result of a bootstrap operation.
///
/// Bootstrap operations modify metadata, not scoped state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapResult {
    /// The new bootstrap metadata after the operation.
    pub new_metadata: BootstrapMetadata,
    /// The audit event recording this operation.
    pub audit_event: AuditEvent,
    /// Optional canonical bid year metadata for `CreateBidYear` operations.
    pub canonical_bid_year: Option<CanonicalBidYear>,
}
