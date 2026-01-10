// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! API request and response data transfer objects.

/// API request to create a new bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBidYearRequest {
    /// The year value (e.g., 2026).
    pub year: u16,
}

/// API response for a successful bid year creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBidYearResponse {
    /// The created bid year.
    pub year: u16,
    /// A success message.
    pub message: String,
}

/// API request to create a new area within a bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAreaRequest {
    /// The bid year this area belongs to.
    pub bid_year: u16,
    /// The area identifier.
    pub area_id: String,
}

/// API response for a successful area creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAreaResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area_id: String,
    /// A success message.
    pub message: String,
}

/// API request to register a new user for a bid year.
///
/// This DTO is distinct from domain types and represents the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUserRequest {
    /// The bid year (e.g., 2026).
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// The user's area identifier.
    pub area: String,
    /// The user's type classification (CPC, CPC-IT, Dev-R, Dev-D).
    pub user_type: String,
    /// The user's crew number (1-7, optional).
    pub crew: Option<u8>,
    /// Cumulative NATCA bargaining unit date (ISO 8601).
    pub cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date (ISO 8601).
    pub natca_bu_date: String,
    /// Entry on Duty / FAA date (ISO 8601).
    pub eod_faa_date: String,
    /// Service Computation Date (ISO 8601).
    pub service_computation_date: String,
    /// Optional lottery value.
    pub lottery_value: Option<u32>,
}

/// API response for a successful user registration.
///
/// This DTO is distinct from domain types and represents the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUserResponse {
    /// The bid year the user was registered for.
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// A success message.
    pub message: String,
}

/// API response for listing bid years.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBidYearsResponse {
    /// The list of bid years.
    pub bid_years: Vec<u16>,
}

/// API request to list areas for a bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAreasRequest {
    /// The bid year to list areas for.
    pub bid_year: u16,
}

/// API response for listing areas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAreasResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The list of area identifiers.
    pub areas: Vec<String>,
}

/// API request to list users for a bid year and area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsersRequest {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
}

/// API response for listing users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsersResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
    /// The list of users.
    pub users: Vec<UserInfo>,
}

/// User information for listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// The user's crew (optional).
    pub crew: Option<u8>,
}
