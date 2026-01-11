// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![deny(
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]

mod auth;
mod error;
mod handlers;
mod request_response;

#[cfg(test)]
mod tests;

// Re-export public types and functions from auth module
pub use auth::{AuthenticatedActor, AuthorizationService, Role, authenticate_stub};

// Re-export public types from error module
pub use error::{ApiError, AuthError, translate_core_error, translate_domain_error};

// Re-export public types from request_response module
pub use request_response::{
    BidYearInfo, CreateAreaRequest, CreateAreaResponse, CreateBidYearRequest,
    CreateBidYearResponse, ListAreasRequest, ListAreasResponse, ListBidYearsResponse,
    ListUsersRequest, ListUsersResponse, RegisterUserRequest, RegisterUserResponse, UserInfo,
};

// Re-export public functions from handlers module
pub use handlers::{
    ApiResult, checkpoint, create_area, create_bid_year, finalize, get_current_state,
    get_historical_state, list_areas, list_bid_years, list_users, register_user, rollback,
};
