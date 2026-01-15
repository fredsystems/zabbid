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
#![allow(deprecated)]
#![allow(clippy::multiple_crate_versions)]

mod auth;
mod capabilities;
mod csv_preview;
mod error;
mod handlers;
mod password_policy;
mod request_response;

#[cfg(test)]
mod tests;

// Re-export public types and functions from auth module
pub use auth::{
    AuthenticatedActor, AuthenticationService, AuthorizationService, Role, authenticate_stub,
};

// Re-export public types from error module
pub use error::{ApiError, AuthError, translate_core_error, translate_domain_error};

// Re-export public types from password_policy module
pub use password_policy::{PasswordPolicy, PasswordPolicyError};

// Re-export public types from request_response module
pub use request_response::{
    AreaCompletenessInfo, AreaInfo, AreaStatusInfo, BidYearCompletenessInfo, BidYearInfo,
    BidYearStatusInfo, BlockingReason, BootstrapAuthStatusResponse, BootstrapLoginRequest,
    BootstrapLoginResponse, BootstrapStatusResponse, Capability, ChangePasswordRequest,
    ChangePasswordResponse, CreateAreaRequest, CreateAreaResponse, CreateBidYearRequest,
    CreateBidYearResponse, CreateFirstAdminRequest, CreateFirstAdminResponse,
    CreateOperatorRequest, CreateOperatorResponse, CsvImportRowResult, CsvImportRowStatus,
    CsvRowPreview, CsvRowStatus, DeleteOperatorRequest, DeleteOperatorResponse,
    DisableOperatorRequest, DisableOperatorResponse, EnableOperatorRequest, EnableOperatorResponse,
    GetActiveBidYearResponse, GetBootstrapCompletenessResponse, GetLeaveAvailabilityRequest,
    GetLeaveAvailabilityResponse, GlobalCapabilities, ImportCsvUsersRequest,
    ImportCsvUsersResponse, ListAreasRequest, ListAreasResponse, ListBidYearsResponse,
    ListOperatorsResponse, ListUsersRequest, ListUsersResponse, LoginRequest, LoginResponse,
    OperatorCapabilities, OperatorInfo, PreviewCsvUsersRequest, PreviewCsvUsersResponse,
    RegisterUserRequest, RegisterUserResponse, ResetPasswordRequest, ResetPasswordResponse,
    SetActiveBidYearRequest, SetActiveBidYearResponse, SetExpectedAreaCountRequest,
    SetExpectedAreaCountResponse, SetExpectedUserCountRequest, SetExpectedUserCountResponse,
    UpdateUserRequest, UpdateUserResponse, UserCapabilities, UserInfo, WhoAmIResponse,
};

// Re-export public functions from capabilities module
pub use capabilities::{
    compute_global_capabilities, compute_operator_capabilities, compute_user_capabilities,
};

// Re-export public functions from handlers module
pub use handlers::{
    ApiResult, RegisterUserResult, bootstrap_login, change_password, check_bootstrap_status,
    checkpoint, create_area, create_bid_year, create_first_admin, create_operator, delete_operator,
    disable_operator, enable_operator, finalize, get_active_bid_year, get_bootstrap_completeness,
    get_bootstrap_status, get_current_state, get_historical_state, get_leave_availability,
    import_csv_users, list_areas, list_bid_years, list_operators, list_users, login, logout,
    preview_csv_users, register_user, reset_password, rollback, set_active_bid_year,
    set_expected_area_count, set_expected_user_count, update_user, whoami,
};
