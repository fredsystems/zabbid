// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Test helper functions and fixtures.

use zab_bid::BootstrapMetadata;
use zab_bid_audit::Cause;
use zab_bid_domain::{Area, BidYear};

use crate::{AuthenticatedActor, RegisterUserRequest, Role};

pub fn create_test_admin() -> AuthenticatedActor {
    AuthenticatedActor::new(String::from("admin-123"), Role::Admin)
}

pub fn create_test_bidder() -> AuthenticatedActor {
    AuthenticatedActor::new(String::from("bidder-456"), Role::Bidder)
}

pub fn create_test_cause() -> Cause {
    Cause::new(String::from("api-req-456"), String::from("API request"))
}

pub fn create_test_metadata() -> BootstrapMetadata {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    metadata.bid_years.push(bid_year.clone());
    metadata.areas.push((bid_year, area));
    metadata
}

pub fn create_valid_request() -> RegisterUserRequest {
    RegisterUserRequest {
        bid_year: 2026,
        initials: String::from("AB"),
        name: String::from("John Doe"),
        area: String::from("North"),
        user_type: String::from("CPC"),
        crew: Some(1),
        cumulative_natca_bu_date: String::from("2019-01-15"),
        natca_bu_date: String::from("2019-06-01"),
        eod_faa_date: String::from("2020-01-15"),
        service_computation_date: String::from("2020-01-15"),
        lottery_value: Some(42),
    }
}
