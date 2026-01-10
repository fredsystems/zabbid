// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{BidYear, DomainError, Initials};

#[test]
fn test_domain_error_display() {
    let bid_year: BidYear = BidYear::new(2026);
    let initials: Initials = Initials::new("AB");

    let err: DomainError = DomainError::DuplicateInitials { bid_year, initials };
    assert_eq!(
        format!("{err}"),
        "User with initials 'AB' already exists in bid year 2026"
    );

    let err: DomainError = DomainError::InvalidInitials(String::from("test"));
    assert_eq!(format!("{err}"), "Invalid initials: test");

    let err: DomainError = DomainError::InvalidName(String::from("test"));
    assert_eq!(format!("{err}"), "Invalid name: test");

    let err: DomainError = DomainError::InvalidArea(String::from("test"));
    assert_eq!(format!("{err}"), "Invalid area: test");

    let err: DomainError = DomainError::InvalidCrew("test");
    assert_eq!(format!("{err}"), "Invalid crew: test");

    let err: DomainError = DomainError::InvalidUserType(String::from("test"));
    assert_eq!(format!("{err}"), "Invalid user type: test");

    let err: DomainError = DomainError::BidYearNotFound(2026);
    assert_eq!(format!("{err}"), "Bid year 2026 not found");

    let err: DomainError = DomainError::AreaNotFound {
        bid_year: 2026,
        area: String::from("North"),
    };
    assert_eq!(format!("{err}"), "Area 'North' not found in bid year 2026");

    let err: DomainError = DomainError::DuplicateBidYear(2026);
    assert_eq!(format!("{err}"), "Bid year 2026 already exists");

    let err: DomainError = DomainError::DuplicateArea {
        bid_year: 2026,
        area: String::from("North"),
    };
    assert_eq!(
        format!("{err}"),
        "Area 'North' already exists in bid year 2026"
    );

    let err: DomainError = DomainError::InvalidBidYear(String::from("test"));
    assert_eq!(format!("{err}"), "Invalid bid year: test");
}
