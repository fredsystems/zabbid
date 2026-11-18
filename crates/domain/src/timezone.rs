// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Authoritative IANA timezone resolution for bid schedules.
//!
//! Timezone data comes from the IANA database bundled into the binary, so
//! resolution is deterministic and independent of the host's zoneinfo.

use crate::error::DomainError;
use jiff::tz::TimeZone;

/// Resolves an IANA timezone identifier (e.g. `America/New_York`).
///
/// Matching is exact and case-sensitive: the identifier must be spelled as it
/// appears in the IANA database. The underlying lookup is case-insensitive, so
/// the resolved name is compared against the input to reject variants such as
/// `america/new_york` that would otherwise be persisted in non-canonical form.
///
/// # Errors
///
/// Returns `DomainError::InvalidTimezone` if the identifier is unknown or is
/// not spelled exactly as in the IANA database.
pub fn resolve_timezone(name: &str) -> Result<TimeZone, DomainError> {
    let tz: TimeZone =
        TimeZone::get(name).map_err(|_| DomainError::InvalidTimezone(name.to_string()))?;

    if tz.iana_name() != Some(name) {
        return Err(DomainError::InvalidTimezone(name.to_string()));
    }

    Ok(tz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_iana_name_resolves() {
        assert!(resolve_timezone("America/New_York").is_ok());
        assert!(resolve_timezone("UTC").is_ok());
    }

    #[test]
    fn unknown_name_is_rejected() {
        assert!(matches!(
            resolve_timezone("Invalid/Timezone"),
            Err(DomainError::InvalidTimezone(name)) if name == "Invalid/Timezone"
        ));
    }

    #[test]
    fn empty_name_is_rejected() {
        assert!(matches!(
            resolve_timezone(""),
            Err(DomainError::InvalidTimezone(_))
        ));
    }

    #[test]
    fn non_canonical_casing_is_rejected() {
        assert!(matches!(
            resolve_timezone("america/new_york"),
            Err(DomainError::InvalidTimezone(name)) if name == "america/new_york"
        ));
    }
}
