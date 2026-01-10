// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::BootstrapMetadata;
use zab_bid_audit::{Actor, Cause};
use zab_bid_domain::{Area, BidYear, SeniorityData};

pub fn create_test_actor() -> Actor {
    Actor::new(String::from("admin-123"), String::from("admin"))
}

pub fn create_test_cause() -> Cause {
    Cause::new(String::from("req-456"), String::from("Admin request"))
}

pub fn create_test_seniority_data() -> SeniorityData {
    SeniorityData::new(
        String::from("2019-01-15"),
        String::from("2019-06-01"),
        String::from("2020-01-15"),
        String::from("2020-01-15"),
        Some(42),
    )
}

pub fn create_test_metadata() -> BootstrapMetadata {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));
    metadata.add_area(BidYear::new(2026), Area::new("North"));
    metadata
}
