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
    clippy::all,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::unwrap_used,
    clippy::expect_used
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
    AdjustBidOrderRequest, AdjustBidOrderResponse, AdjustBidWindowRequest, AdjustBidWindowResponse,
    AreaCompletenessInfo, AreaInfo, AreaStatusInfo, BidOrderAdjustment, BidScheduleInfo,
    BidStatusHistoryInfo, BidStatusInfo, BidYearCompletenessInfo, BidYearInfo, BidYearStatusInfo,
    BlockingReason, BootstrapAuthStatusResponse, BootstrapLoginRequest, BootstrapLoginResponse,
    BootstrapStatusResponse, BulkUpdateBidStatusRequest, BulkUpdateBidStatusResponse, Capability,
    ChangePasswordRequest, ChangePasswordResponse, ConfirmReadyToBidRequest,
    ConfirmReadyToBidResponse, CreateAreaRequest, CreateAreaResponse, CreateBidYearRequest,
    CreateBidYearResponse, CreateFirstAdminRequest, CreateFirstAdminResponse,
    CreateOperatorRequest, CreateOperatorResponse, CreateRoundGroupRequest,
    CreateRoundGroupResponse, CreateRoundRequest, CreateRoundResponse, CsvImportRowResult,
    CsvImportRowStatus, CsvRowPreview, CsvRowStatus, DeleteOperatorRequest, DeleteOperatorResponse,
    DeleteRoundGroupResponse, DeleteRoundResponse, DisableOperatorRequest, DisableOperatorResponse,
    EnableOperatorRequest, EnableOperatorResponse, GetActiveBidYearResponse,
    GetBidOrderPreviewResponse, GetBidScheduleResponse, GetBidStatusForAreaRequest,
    GetBidStatusForAreaResponse, GetBidStatusRequest, GetBidStatusResponse,
    GetBidYearReadinessResponse, GetBootstrapCompletenessResponse, GetLeaveAvailabilityRequest,
    GetLeaveAvailabilityResponse, GlobalCapabilities, ImportCsvUsersRequest,
    ImportCsvUsersResponse, ListAreasRequest, ListAreasResponse, ListBidYearsResponse,
    ListOperatorsResponse, ListRoundGroupsResponse, ListRoundsResponse, ListUsersRequest,
    ListUsersResponse, LoginRequest, LoginResponse, OperatorCapabilities, OperatorInfo,
    OverrideAreaAssignmentRequest, OverrideAreaAssignmentResponse, OverrideBidOrderRequest,
    OverrideBidOrderResponse, OverrideBidWindowRequest, OverrideBidWindowResponse,
    OverrideEligibilityRequest, OverrideEligibilityResponse, PreviewCsvUsersRequest,
    PreviewCsvUsersResponse, RecalculateBidWindowsRequest, RecalculateBidWindowsResponse,
    RegisterUserRequest, RegisterUserResponse, ResetPasswordRequest, ResetPasswordResponse,
    ReviewNoBidUserResponse, RoundGroupInfo, RoundInfo, SetActiveBidYearRequest,
    SetActiveBidYearResponse, SetBidScheduleRequest, SetBidScheduleResponse,
    SetExpectedAreaCountRequest, SetExpectedAreaCountResponse, SetExpectedUserCountRequest,
    SetExpectedUserCountResponse, TransitionBidStatusRequest, TransitionBidStatusResponse,
    TransitionToBiddingActiveRequest, TransitionToBiddingActiveResponse,
    TransitionToBiddingClosedRequest, TransitionToBiddingClosedResponse,
    TransitionToBootstrapCompleteRequest, TransitionToBootstrapCompleteResponse,
    TransitionToCanonicalizedRequest, TransitionToCanonicalizedResponse, UpdateAreaRequest,
    UpdateAreaResponse, UpdateBidYearMetadataRequest, UpdateBidYearMetadataResponse,
    UpdateRoundGroupRequest, UpdateRoundGroupResponse, UpdateRoundRequest, UpdateRoundResponse,
    UpdateUserParticipationRequest, UpdateUserParticipationResponse, UpdateUserRequest,
    UpdateUserResponse, UserCapabilities, UserInfo, WhoAmIResponse,
};

// Re-export public functions from capabilities module
pub use capabilities::{
    compute_global_capabilities, compute_operator_capabilities, compute_user_capabilities,
};

// Re-export public functions from handlers module
pub use handlers::{
    ApiResult, RegisterUserResult, adjust_bid_order, adjust_bid_window, bootstrap_login,
    bulk_update_bid_status, change_password, check_bootstrap_status, checkpoint,
    confirm_ready_to_bid, create_area, create_bid_year, create_first_admin, create_operator,
    create_round, create_round_group, delete_operator, delete_round, delete_round_group,
    disable_operator, enable_operator, finalize, get_active_bid_year, get_bid_order_preview,
    get_bid_schedule, get_bid_status, get_bid_status_for_area, get_bid_year_readiness,
    get_bootstrap_completeness, get_bootstrap_status, get_current_state, get_historical_state,
    get_leave_availability, import_csv_users, list_areas, list_bid_years, list_operators,
    list_round_groups, list_rounds, list_users, login, logout, override_area_assignment,
    override_bid_order, override_bid_window, override_eligibility, preview_csv_users,
    recalculate_bid_windows, register_user, reset_password, review_no_bid_user, rollback,
    set_active_bid_year, set_bid_schedule, set_expected_area_count, set_expected_user_count,
    transition_bid_status, transition_to_bidding_active, transition_to_bidding_closed,
    transition_to_bootstrap_complete, transition_to_canonicalized, update_area,
    update_bid_year_metadata, update_round, update_round_group, update_user,
    update_user_participation, whoami,
};
