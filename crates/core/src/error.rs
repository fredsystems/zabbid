// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use zab_bid_domain::DomainError;

/// Errors that can occur during state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// A domain rule was violated.
    DomainViolation(DomainError),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DomainViolation(err) => write!(f, "Domain violation: {err}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<DomainError> for CoreError {
    fn from(err: DomainError) -> Self {
        Self::DomainViolation(err)
    }
}
