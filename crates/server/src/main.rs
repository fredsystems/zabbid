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
#![allow(clippy::multiple_crate_versions)]

mod live;
mod session;

use axum::{
    Json, Router,
    extract::{Path, Query, State as AxumState},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use clap::Parser;
use live::{LiveEvent, LiveEventBroadcaster};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_api::{
    AdjustBidOrderRequest, AdjustBidOrderResponse, AdjustBidWindowRequest, AdjustBidWindowResponse,
    ApiError, ApiResult, BidOrderAdjustment, BootstrapStatusResponse, ConfirmReadyToBidRequest,
    ConfirmReadyToBidResponse, CreateAreaRequest, CreateAreaResponse, CreateBidYearRequest,
    CreateBidYearResponse, CreateRoundGroupRequest, CreateRoundGroupResponse, CreateRoundRequest,
    CreateRoundResponse, CsvImportRowStatus, DeleteRoundGroupResponse, DeleteRoundResponse,
    GetActiveBidYearResponse, GetBidOrderPreviewResponse, GetBidScheduleResponse,
    GetBidYearReadinessResponse, GetBootstrapCompletenessResponse, GetLeaveAvailabilityResponse,
    ImportCsvUsersRequest, ImportCsvUsersResponse, ListAreasRequest, ListAreasResponse,
    ListBidYearsResponse, ListRoundGroupsResponse, ListRoundsResponse, ListUsersResponse,
    OverrideAreaAssignmentRequest, OverrideAreaAssignmentResponse, OverrideBidOrderRequest,
    OverrideBidOrderResponse, OverrideBidWindowRequest, OverrideBidWindowResponse,
    OverrideEligibilityRequest, OverrideEligibilityResponse, PreviewCsvUsersRequest,
    PreviewCsvUsersResponse, RecalculateBidWindowsRequest, RecalculateBidWindowsResponse,
    RegisterUserRequest, RegisterUserResponse, RegisterUserResult, ReviewNoBidUserResponse,
    SetActiveBidYearRequest, SetActiveBidYearResponse, SetBidScheduleRequest,
    SetBidScheduleResponse, SetExpectedAreaCountRequest, SetExpectedAreaCountResponse,
    SetExpectedUserCountRequest, SetExpectedUserCountResponse, TransitionToBiddingActiveRequest,
    TransitionToBiddingActiveResponse, TransitionToBiddingClosedRequest,
    TransitionToBiddingClosedResponse, TransitionToBootstrapCompleteRequest,
    TransitionToBootstrapCompleteResponse, TransitionToCanonicalizedRequest,
    TransitionToCanonicalizedResponse, UpdateAreaRequest, UpdateAreaResponse,
    UpdateBidYearMetadataRequest, UpdateBidYearMetadataResponse, UpdateRoundGroupRequest,
    UpdateRoundGroupResponse, UpdateRoundRequest, UpdateRoundResponse,
    UpdateUserParticipationRequest, UpdateUserParticipationResponse, UpdateUserRequest,
    UpdateUserResponse, adjust_bid_order, adjust_bid_window, checkpoint, confirm_ready_to_bid,
    create_area, create_bid_year, create_round, create_round_group, delete_round,
    delete_round_group, finalize, get_active_bid_year, get_bid_order_preview, get_bid_schedule,
    get_bid_year_readiness, get_bootstrap_completeness, get_bootstrap_status, get_current_state,
    get_historical_state, get_leave_availability, import_csv_users, list_areas, list_bid_years,
    list_round_groups, list_rounds, list_users, override_area_assignment, override_bid_order,
    override_bid_window, override_eligibility, preview_csv_users, recalculate_bid_windows,
    register_user, review_no_bid_user, rollback, set_active_bid_year, set_bid_schedule,
    set_expected_area_count, set_expected_user_count, transition_to_bidding_active,
    transition_to_bidding_closed, transition_to_bootstrap_complete, transition_to_canonicalized,
    update_area, update_bid_year_metadata, update_round, update_round_group, update_user,
    update_user_participation,
};
use zab_bid_audit::{AuditEvent, Cause};
use zab_bid_domain::{Area, BidYear, BidYearLifecycle, CanonicalBidYear, Initials};
use zab_bid_persistence::{Persistence, PersistenceError};

/// ZAB Bid Server - HTTP server for the ZAB Bidding System
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Database backend to use (sqlite or mysql)
    #[arg(long, default_value = "sqlite")]
    db_backend: String,

    /// Path to the `SQLite` database file. If not provided, uses in-memory database.
    /// Only valid when --db-backend=sqlite.
    #[arg(short, long)]
    database: Option<String>,

    /// `MySQL` database URL (required when --db-backend=mysql)
    #[arg(long)]
    database_url: Option<String>,

    /// Port to bind the server to
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

impl Args {
    /// Validates argument combinations based on selected backend.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unknown backend is specified
    /// - `MySQL` backend is selected without --database-url
    /// - `SQLite` backend is used with --database-url
    /// - `MySQL` backend is used with --database
    fn validate(&self) -> Result<(), String> {
        match self.db_backend.as_str() {
            "sqlite" => {
                if self.database_url.is_some() {
                    return Err(
                        "SQLite backend does not support --database-url. Use --database instead."
                            .to_string(),
                    );
                }
                Ok(())
            }
            "mysql" => {
                if self.database_url.is_none() {
                    return Err("MySQL backend requires --database-url".to_string());
                }
                if self.database.is_some() {
                    return Err(
                        "MySQL backend does not support --database. Use --database-url instead."
                            .to_string(),
                    );
                }
                Ok(())
            }
            unknown => Err(format!(
                "Unknown database backend: '{unknown}'. Valid options: sqlite, mysql"
            )),
        }
    }
}

/// Application state shared across handlers.
///
/// This contains the persistence layer wrapped in a Mutex to allow
/// safe concurrent access, and a live event broadcaster for WebSocket streaming.
#[derive(Clone)]
struct AppState {
    /// The persistence layer for audit events and state snapshots.
    persistence: Arc<Mutex<Persistence>>,
    /// Live event broadcaster for streaming state changes to connected clients.
    live_events: Arc<LiveEventBroadcaster>,
}

/// API request for registering a user.
///
/// Authentication is now handled via session token in Authorization header.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RegisterUserApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The user's initials.
    initials: String,
    /// The user's name.
    name: String,
    /// The user's area canonical ID.
    area_id: i64,
    /// The user's type classification.
    user_type: String,
    /// The user's crew identifier.
    crew: Option<u8>,
    /// Cumulative NATCA bargaining unit date (ISO 8601).
    cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date (ISO 8601).
    natca_bu_date: String,
    /// Entry on Duty / FAA date (ISO 8601).
    eod_faa_date: String,
    /// Service Computation Date (ISO 8601).
    service_computation_date: String,
    /// Optional lottery value.
    lottery_value: Option<u32>,
}

/// API request for checkpoint, finalize, or rollback operations.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct AdminActionRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical area identifier.
    area_id: i64,
    /// The target event ID (only for rollback).
    #[serde(skip_serializing_if = "Option::is_none")]
    target_event_id: Option<i64>,
}

/// API request for creating a bid year.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct CreateBidYearApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The year value (e.g., 2026).
    year: u16,
    /// The start date of the bid year (ISO 8601).
    start_date: String,
    /// The number of pay periods (must be 26 or 27).
    num_pay_periods: u8,
}

/// API request for creating an area.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct CreateAreaApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The area identifier.
    area_id: String,
}

/// Query parameters for listing areas.
#[derive(Debug, Deserialize)]
struct ListAreasQuery {
    /// The canonical bid year identifier.
    bid_year_id: i64,
}

/// Query parameters for listing users.
#[derive(Debug, Deserialize)]
struct ListUsersQuery {
    /// The canonical area identifier.
    area_id: i64,
}

/// Query parameters for leave availability.
#[derive(Debug, Clone, Deserialize)]
struct LeaveAvailabilityQuery {
    /// The canonical user identifier.
    user_id: i64,
}

/// API response for write operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WriteResponse {
    /// Success indicator.
    success: bool,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    /// The event ID of the persisted audit event.
    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<i64>,
}

/// Query parameters for current state endpoint.
#[derive(Debug, Deserialize)]
struct CurrentStateQuery {
    /// The canonical area identifier.
    area_id: i64,
}

/// Query parameters for historical state endpoint.
#[derive(Debug, Deserialize)]
struct HistoricalStateQuery {
    /// The canonical area identifier.
    area_id: i64,
    /// The timestamp (ISO 8601 format).
    timestamp: String,
}

/// Query parameters for audit timeline endpoint.
#[derive(Debug, Deserialize)]
struct AuditTimelineQuery {
    /// The canonical area identifier.
    area_id: i64,
}

/// Serializable representation of State for JSON responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateResponse {
    /// The canonical bid year identifier.
    bid_year_id: i64,
    /// The bid year.
    bid_year: u16,
    /// The canonical area identifier.
    area_id: i64,
    /// The area code.
    area_code: String,
    /// The users in this state.
    users: Vec<UserResponse>,
}

/// Serializable representation of a User for JSON responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserResponse {
    /// The bid year.
    bid_year: u16,
    /// The user's initials.
    initials: String,
    /// The user's name.
    name: String,
    /// The area.
    area: String,
    /// The crew.
    crew: String,
    /// Cumulative NATCA bargaining unit date.
    cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date.
    natca_bu_date: String,
    /// Entry on Duty / FAA date.
    eod_faa_date: String,
    /// Service Computation Date.
    service_computation_date: String,
    /// Optional lottery value.
    lottery_value: Option<u32>,
}

/// Serializable representation of an `AuditEvent` for JSON responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEventResponse {
    /// The event ID.
    event_id: Option<i64>,
    /// The actor ID.
    actor_id: String,
    /// The actor type.
    actor_type: String,
    /// The cause ID.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The action name.
    action_name: String,
    /// Optional action details.
    action_details: Option<String>,
    /// State before the transition.
    before_snapshot: String,
    /// State after the transition.
    after_snapshot: String,
    /// The bid year (optional for global events).
    bid_year: Option<u16>,
    /// The area (optional for global events).
    area: Option<String>,
}

/// Error response type.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorResponse {
    /// Error indicator.
    error: bool,
    /// Error message.
    message: String,
}

/// HTTP error wrapper that implements `IntoResponse`.
struct HttpError {
    /// The HTTP status code.
    status: StatusCode,
    /// The error message.
    message: String,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let body: Json<ErrorResponse> = Json(ErrorResponse {
            error: true,
            message: self.message,
        });
        (self.status, body).into_response()
    }
}

impl From<ApiError> for HttpError {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::AuthenticationFailed { .. } => Self {
                status: StatusCode::UNAUTHORIZED,
                message: err.to_string(),
            },
            ApiError::Unauthorized { .. } => Self {
                status: StatusCode::FORBIDDEN,
                message: err.to_string(),
            },
            ApiError::DomainRuleViolation { .. } => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                message: err.to_string(),
            },
            ApiError::InvalidInput { .. }
            | ApiError::PasswordPolicyViolation { .. }
            | ApiError::InvalidCsvFormat { .. } => Self {
                status: StatusCode::BAD_REQUEST,
                message: err.to_string(),
            },
            ApiError::ResourceNotFound { .. } => Self {
                status: StatusCode::NOT_FOUND,
                message: err.to_string(),
            },
            ApiError::Internal { .. } => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}

impl From<PersistenceError> for HttpError {
    fn from(err: PersistenceError) -> Self {
        error!(error = %err, "Persistence error");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Persistence error: {err}"),
        }
    }
}

/// Converts a `State` to a `StateResponse`.
fn state_to_response(
    state: &State,
    metadata: &BootstrapMetadata,
) -> Result<StateResponse, HttpError> {
    // Extract bid_year_id from metadata
    let bid_year_id: i64 = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == state.bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!(
                "Bid year {} exists but has no ID in metadata",
                state.bid_year.year()
            ),
        })?;

    // Extract area_id from metadata
    let area_id: i64 = metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == state.bid_year.year())
        .find(|(_, a)| a.area_code() == state.area.id())
        .and_then(|(_, a)| a.area_id())
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!(
                "Area '{}' in bid year {} exists but has no ID in metadata",
                state.area.id(),
                state.bid_year.year()
            ),
        })?;

    Ok(StateResponse {
        bid_year_id,
        bid_year: state.bid_year.year(),
        area_id,
        area_code: state.area.id().to_string(),
        users: state
            .users
            .iter()
            .map(|user| UserResponse {
                bid_year: user.bid_year.year(),
                initials: user.initials.value().to_string(),
                name: user.name.clone(),
                area: user.area.id().to_string(),
                crew: user
                    .crew
                    .map_or_else(String::new, |c| c.number().to_string()),
                cumulative_natca_bu_date: user.seniority_data.cumulative_natca_bu_date.clone(),
                natca_bu_date: user.seniority_data.natca_bu_date.clone(),
                eod_faa_date: user.seniority_data.eod_faa_date.clone(),
                service_computation_date: user.seniority_data.service_computation_date.clone(),
                lottery_value: user.seniority_data.lottery_value,
            })
            .collect(),
    })
}

/// Converts an `AuditEvent` to an `AuditEventResponse`.
fn audit_event_to_response(event: &AuditEvent) -> AuditEventResponse {
    AuditEventResponse {
        event_id: event.event_id,
        actor_id: event.actor.id.clone(),
        actor_type: event.actor.actor_type.clone(),
        cause_id: event.cause.id.clone(),
        cause_description: event.cause.description.clone(),
        action_name: event.action.name.clone(),
        action_details: event.action.details.clone(),
        before_snapshot: event.before.data.clone(),
        after_snapshot: event.after.data.clone(),
        bid_year: event.bid_year.as_ref().map(BidYear::year),
        area: event.area.as_ref().map(|a| a.id().to_string()),
    }
}

/// API request wrapper for lifecycle transition to `BootstrapComplete`.
#[derive(Debug, serde::Deserialize)]
struct TransitionToBootstrapCompleteApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical bid year identifier.
    bid_year_id: i64,
}

/// API request wrapper for lifecycle transition to `Canonicalized`.
#[derive(Debug, serde::Deserialize)]
struct TransitionToCanonicalizedApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical bid year identifier.
    bid_year_id: i64,
}

/// API request wrapper for lifecycle transition to `BiddingActive`.
#[derive(Debug, serde::Deserialize)]
struct TransitionToBiddingActiveApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical bid year identifier.
    bid_year_id: i64,
}

/// API request wrapper for lifecycle transition to `BiddingClosed`.
#[derive(Debug, serde::Deserialize)]
struct TransitionToBiddingClosedApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical bid year identifier.
    bid_year_id: i64,
}

/// API request wrapper for updating bid year metadata.
#[derive(Debug, serde::Deserialize)]
struct UpdateBidYearMetadataApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical bid year identifier.
    bid_year_id: i64,
    /// Optional display label (max 100 characters).
    label: Option<String>,
    /// Optional operational notes (max 2000 characters).
    notes: Option<String>,
}

/// Handler for POST `/bid_years` endpoint.
///
/// Creates a new bid year.
async fn handle_create_bid_year(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<CreateBidYearApiRequest>,
) -> Result<Json<CreateBidYearResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        year = req.year,
        "Handling create_bid_year request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    drop(persistence);

    // Build API request
    // Parse start date from ISO 8601 string
    let start_date: time::Date = time::Date::parse(
        &req.start_date,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|e| HttpError {
        status: StatusCode::BAD_REQUEST,
        message: format!("Invalid start_date format: {e}"),
    })?;

    let create_request: CreateBidYearRequest = CreateBidYearRequest {
        year: req.year,
        start_date,
        num_pay_periods: req.num_pay_periods,
    };

    // Execute command via API
    let bootstrap_result: BootstrapResult =
        create_bid_year(&metadata, &create_request, &actor, &operator, cause)?;

    // Persist the bootstrap result
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_bootstrap(&bootstrap_result)?;

    // Get updated metadata to retrieve the canonical bid_year_id
    let updated_metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Phase 25B: Auto-create No Bid system area
    let bid_year_id: i64 = updated_metadata
        .bid_years
        .iter()
        .find(|by| by.year() == req.year)
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to retrieve bid_year_id for year {}", req.year),
        })?;

    let no_bid_area_id: i64 = persistence
        .create_system_area(bid_year_id, Area::NO_BID_AREA_CODE)
        .map_err(|e| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to create No Bid area: {e}"),
        })?;

    info!(no_bid_area_id, bid_year_id, "Created No Bid system area");

    drop(persistence);

    info!(
        event_id = event_id,
        bid_year_id = bid_year_id,
        year = req.year,
        "Successfully created bid year"
    );

    // Broadcast live event
    app_state
        .live_events
        .broadcast(&LiveEvent::BidYearCreated { year: req.year });

    // Return detailed response
    let end_date: time::Date = bootstrap_result
        .canonical_bid_year
        .as_ref()
        .and_then(|by| by.end_date().ok())
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Failed to calculate end_date".to_string(),
        })?;

    Ok(Json(CreateBidYearResponse {
        bid_year_id,
        year: req.year,
        start_date,
        num_pay_periods: req.num_pay_periods,
        end_date,
        message: format!("Created bid year {}", req.year),
    }))
}

/// Handler for POST `/areas` endpoint.
///
/// Creates a new area within a bid year.
async fn handle_create_area(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<CreateAreaApiRequest>,
) -> Result<Json<CreateAreaResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = %req.area_id,
        "Handling create_area request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata and persistence
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Build API request
    let create_request: CreateAreaRequest = CreateAreaRequest {
        area_id: req.area_id.clone(),
    };

    // Execute command via API
    let bootstrap_result: BootstrapResult = create_area(
        &mut persistence,
        &metadata,
        &create_request,
        &actor,
        &operator,
        cause,
    )?;

    // Persist the bootstrap result
    let event_id: i64 = persistence.persist_bootstrap(&bootstrap_result)?;

    // Get updated metadata to retrieve the canonical area_id
    let updated_metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let bid_year_ref = bootstrap_result
        .audit_event
        .bid_year
        .as_ref()
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("CreateArea event missing bid year"),
        })?;

    let area_id: i64 = updated_metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == bid_year_ref.year())
        .find(|(_, a)| a.area_code() == req.area_id.to_uppercase())
        .and_then(|(_, a)| a.area_id())
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to retrieve area_id for area {}", req.area_id),
        })?;

    drop(persistence);

    info!(
        event_id = event_id,
        area_id = area_id,
        area_code = %req.area_id,
        "Successfully created area"
    );

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::AreaCreated {
        bid_year: bid_year_ref.year(),
        area: req.area_id.clone(),
    });

    #[allow(clippy::redundant_closure_for_method_calls)]
    let bid_year_id: i64 = updated_metadata
        .bid_years
        .iter()
        .find(|by| by.year() == bid_year_ref.year())
        .and_then(|by| by.bid_year_id())
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Active bid year missing ID"),
        })?;

    Ok(Json(CreateAreaResponse {
        bid_year_id,
        bid_year: bid_year_ref.year(),
        area_id,
        area_code: req.area_id.clone(),
        message: format!(
            "Created area '{}' in bid year {}",
            req.area_id,
            bid_year_ref.year()
        ),
    }))
}

/// Handler for GET `/bid_years` endpoint.
///
/// Lists all bid years.
async fn handle_list_bid_years(
    AxumState(app_state): AxumState<AppState>,
) -> Result<Json<ListBidYearsResponse>, HttpError> {
    info!("Handling list_bid_years request");

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years()?;

    // Get aggregate counts
    let area_counts: Vec<(u16, usize)> = persistence.count_areas_by_bid_year()?;
    let user_counts: Vec<(u16, usize)> = persistence.count_users_by_bid_year()?;
    let mut response: ListBidYearsResponse =
        list_bid_years(&mut persistence, &metadata, &canonical_bid_years)?;

    drop(persistence);

    // Enrich with counts
    for info in &mut response.bid_years {
        info.area_count = area_counts
            .iter()
            .find(|(year, _)| *year == info.year)
            .map_or(0, |(_, count)| *count);
        info.total_user_count = user_counts
            .iter()
            .find(|(year, _)| *year == info.year)
            .map_or(0, |(_, count)| *count);
    }

    Ok(Json(response))
}

/// Handler for GET `/areas` endpoint.
///
/// Lists all areas for a given bid year.
async fn handle_list_areas(
    AxumState(app_state): AxumState<AppState>,
    Query(query): Query<ListAreasQuery>,
) -> Result<Json<ListAreasResponse>, HttpError> {
    info!(
        bid_year_id = query.bid_year_id,
        "Handling list_areas request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &zab_bid_domain::BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(query.bid_year_id))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Bid year with ID {} not found", query.bid_year_id),
        })?;

    // Get user counts per area
    let user_counts: Vec<(String, usize)> = persistence.count_users_by_area(bid_year)?;
    drop(persistence);

    let request: ListAreasRequest = ListAreasRequest {
        bid_year_id: query.bid_year_id,
    };
    let mut response: ListAreasResponse = list_areas(&metadata, &request)?;

    // Enrich with user counts
    for area_info in &mut response.areas {
        area_info.user_count = user_counts
            .iter()
            .find(|(area_code, _)| area_code == &area_info.area_code)
            .map_or(0, |(_, count)| *count);
    }

    Ok(Json(response))
}

/// Handler for GET `/users` endpoint.
///
/// Lists all users for a given bid year and area with user capabilities.
async fn handle_list_users(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, HttpError> {
    info!(area_id = query.area_id, "Handling list_users request");

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(query.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", query.area_id),
        })?;

    // Extract bid_year_id for lifecycle state lookup
    let bid_year_id: i64 = bid_year.bid_year_id().ok_or_else(|| HttpError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!(
            "Bid year {} exists but has no ID in metadata",
            bid_year.year()
        ),
    })?;

    // Fetch lifecycle state from persistence
    let lifecycle_state_str: String = persistence.get_lifecycle_state(bid_year_id)?;
    let lifecycle_state: BidYearLifecycle = lifecycle_state_str.parse().map_err(|e| HttpError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("Failed to parse lifecycle state: {e}"),
    })?;

    let canonical_bid_years: Vec<CanonicalBidYear> = persistence.list_bid_years()?;
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    let response: ListUsersResponse = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &state,
        &actor,
        &operator,
        lifecycle_state,
    )?;

    Ok(Json(response))
}

/// Handler for GET `/leave/availability` endpoint.
///
/// Returns leave availability for a specific user.
async fn handle_get_leave_availability(
    AxumState(app_state): AxumState<AppState>,
    Query(query): Query<LeaveAvailabilityQuery>,
) -> Result<Json<GetLeaveAvailabilityResponse>, HttpError> {
    info!(
        user_id = query.user_id,
        "Handling get_leave_availability request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Find the user by user_id across all areas
    let canonical_bid_years: Vec<CanonicalBidYear> = persistence.list_bid_years()?;

    // Search all states to find the user
    let mut found_user: Option<(BidYear, Area, Initials, &CanonicalBidYear, State)> = None;

    for (bid_year_domain, area_domain) in &metadata.areas {
        if let Ok(state) = persistence.get_current_state(bid_year_domain, area_domain)
            && let Some(user) = state
                .users
                .iter()
                .find(|u| u.user_id == Some(query.user_id))
            && let Some(canonical_by) = canonical_bid_years
                .iter()
                .find(|cby| cby.year() == bid_year_domain.year())
        {
            found_user = Some((
                bid_year_domain.clone(),
                area_domain.clone(),
                user.initials.clone(),
                canonical_by,
                state,
            ));
            break;
        }
    }
    drop(persistence);

    let (_bid_year, area, initials, canonical_bid_year, state) =
        found_user.ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("User with ID {} not found", query.user_id),
        })?;

    let response: GetLeaveAvailabilityResponse =
        get_leave_availability(&metadata, canonical_bid_year, &area, &initials, &state)?;

    Ok(Json(response))
}

/// Handler for POST `/register_user` endpoint.
///
/// Authenticates the actor, authorizes the action, and registers a new user.
async fn handle_register_user(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<RegisterUserApiRequest>,
) -> Result<Json<RegisterUserResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = req.area_id,
        initials = %req.initials,
        "Handling register_user request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(req.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", req.area_id),
        })?;

    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));

    // Build API request
    let register_request: RegisterUserRequest = RegisterUserRequest {
        initials: req.initials,
        name: req.name,
        area: area.area_code().to_string(),
        user_type: req.user_type,
        crew: req.crew,
        cumulative_natca_bu_date: req.cumulative_natca_bu_date,
        natca_bu_date: req.natca_bu_date,
        eod_faa_date: req.eod_faa_date,
        service_computation_date: req.service_computation_date,
        lottery_value: req.lottery_value,
    };

    // Execute command via API
    let result: ApiResult<RegisterUserResult> = register_user(
        &mut persistence,
        &metadata,
        &state,
        register_request,
        &actor,
        &operator,
        cause,
    )?;

    // Persist the transition (persistence already locked)
    let transition_result: TransitionResult = TransitionResult {
        audit_event: result.audit_event.clone(),
        new_state: result.new_state.clone(),
    };
    let persist_result = persistence.persist_transition(&transition_result)?;
    let event_id: i64 = persist_result.event_id;

    // Extract bid_year_id from metadata
    let bid_year_id: i64 = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!(
                "Bid year {} exists but has no ID in metadata",
                bid_year.year()
            ),
        })?;

    // Get user_id from persist result (guaranteed to be present for RegisterUser)
    let user_id: i64 = persist_result.user_id.ok_or_else(|| HttpError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: "RegisterUser transition did not return user_id".to_string(),
    })?;

    drop(persistence);

    info!(
        event_id = event_id,
        user_id = user_id,
        bid_year_id = bid_year_id,
        initials = %result.response.initials,
        "Successfully registered user"
    );

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::UserRegistered {
        bid_year: result.response.bid_year,
        area: area.area_code().to_string(),
        initials: result.response.initials.clone(),
    });

    // Construct final API response with all IDs populated
    Ok(Json(RegisterUserResponse {
        bid_year_id,
        bid_year: result.response.bid_year,
        user_id,
        initials: result.response.initials,
        name: result.response.name,
        message: result.response.message,
        event_id,
    }))
}

/// Handler for POST /checkpoint endpoint.
///
/// Authenticates the actor, authorizes the action, and creates a checkpoint.
async fn handle_checkpoint(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<AdminActionRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = req.area_id,
        "Handling checkpoint request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(req.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", req.area_id),
        })?;

    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));

    // Execute command via API (persistence passed for active bid year resolution)
    let result: TransitionResult = checkpoint(
        &mut persistence,
        &metadata,
        &state,
        &actor,
        &operator,
        cause,
    )?;

    // Persist the transition
    let persist_result = persistence.persist_transition(&result)?;
    let event_id: i64 = persist_result.event_id;
    drop(persistence);

    info!(event_id = event_id, "Successfully created checkpoint");

    // Broadcast live event
    app_state
        .live_events
        .broadcast(&LiveEvent::CheckpointCreated {
            bid_year: bid_year.year(),
            area: area.area_code().to_string(),
        });

    Ok(Json(WriteResponse {
        success: true,
        message: Some(String::from("Checkpoint created successfully")),
        event_id: Some(event_id),
    }))
}

/// Handler for POST /finalize endpoint.
///
/// Authenticates the actor, authorizes the action, and finalizes a round.
async fn handle_finalize(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<AdminActionRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = req.area_id,
        "Handling finalize request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(req.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", req.area_id),
        })?;

    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));

    // Execute command via API (persistence passed for active bid year resolution)
    let result: TransitionResult = finalize(
        &mut persistence,
        &metadata,
        &state,
        &actor,
        &operator,
        cause,
    )?;

    // Persist the transition
    let persist_result = persistence.persist_transition(&result)?;
    let event_id: i64 = persist_result.event_id;
    drop(persistence);

    info!(event_id = event_id, "Successfully finalized round");

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::RoundFinalized {
        bid_year: bid_year.year(),
        area: area.area_code().to_string(),
    });

    Ok(Json(WriteResponse {
        success: true,
        message: Some(String::from("Round finalized successfully")),
        event_id: Some(event_id),
    }))
}

/// Handler for POST /rollback endpoint.
///
/// Authenticates the actor, authorizes the action, and rolls back to a specific event.
async fn handle_rollback(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<AdminActionRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = req.area_id,
        target_event_id = ?req.target_event_id,
        "Handling rollback request"
    );

    let target_event_id: i64 = req.target_event_id.ok_or_else(|| HttpError {
        status: StatusCode::BAD_REQUEST,
        message: String::from("target_event_id is required for rollback"),
    })?;

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(req.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", req.area_id),
        })?;

    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));

    // Execute command via API (persistence passed for active bid year resolution)
    let result: TransitionResult = rollback(
        &mut persistence,
        &metadata,
        &state,
        target_event_id,
        &actor,
        &operator,
        cause,
    )?;

    // Persist the transition
    let persist_result = persistence.persist_transition(&result)?;
    let event_id: i64 = persist_result.event_id;
    drop(persistence);

    info!(
        event_id = event_id,
        target_event_id = target_event_id,
        "Successfully rolled back to event"
    );

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::RolledBack {
        bid_year: bid_year.year(),
        area: area.area_code().to_string(),
    });

    Ok(Json(WriteResponse {
        success: true,
        message: Some(format!(
            "Successfully rolled back to event {target_event_id}"
        )),
        event_id: Some(event_id),
    }))
}

/// Handler for GET /state/current endpoint.
///
/// Returns the current effective state for a given bid year and area.
async fn handle_get_current_state(
    AxumState(app_state): AxumState<AppState>,
    Query(params): Query<CurrentStateQuery>,
) -> Result<Json<StateResponse>, HttpError> {
    info!(
        area_id = params.area_id,
        "Handling get_current_state request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(params.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", params.area_id),
        })?;

    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    let validated_state: State = get_current_state(&metadata, &bid_year, &area, state)?;
    let response: StateResponse = state_to_response(&validated_state, &metadata)?;

    Ok(Json(response))
}

/// Handler for GET /state/historical endpoint.
///
/// Returns the historical state for a given bid year, area, and timestamp.
async fn handle_get_historical_state(
    AxumState(app_state): AxumState<AppState>,
    Query(params): Query<HistoricalStateQuery>,
) -> Result<Json<StateResponse>, HttpError> {
    info!(
        area_id = params.area_id,
        timestamp = %params.timestamp,
        "Handling get_historical_state request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(params.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", params.area_id),
        })?;

    let state: State = persistence.get_historical_state(&bid_year, &area, &params.timestamp)?;
    drop(persistence);

    let validated_state: State = get_historical_state(&metadata, &bid_year, &area, state)?;
    let response: StateResponse = state_to_response(&validated_state, &metadata)?;

    Ok(Json(response))
}

/// Handler for GET /audit/timeline endpoint.
///
/// Returns the ordered audit event timeline for a given bid year and area.
async fn handle_get_audit_timeline(
    AxumState(app_state): AxumState<AppState>,
    Query(params): Query<AuditTimelineQuery>,
) -> Result<Json<Vec<AuditEventResponse>>, HttpError> {
    info!(
        area_id = params.area_id,
        "Handling get_audit_timeline request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve area_id to Area and BidYear from metadata
    let (bid_year, area) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(params.area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Area with ID {} not found", params.area_id),
        })?;

    let events: Vec<AuditEvent> = persistence.get_audit_timeline(&bid_year, &area)?;
    drop(persistence);

    let response: Vec<AuditEventResponse> = events.iter().map(audit_event_to_response).collect();

    Ok(Json(response))
}

/// Handler for GET `/audit/event/{event_id}` endpoint.
///
/// Returns a specific audit event by its ID.
async fn handle_get_audit_event(
    AxumState(app_state): AxumState<AppState>,
    Path(event_id): Path<i64>,
) -> Result<Json<AuditEventResponse>, HttpError> {
    info!(event_id = event_id, "Handling get_audit_event request");

    let mut persistence = app_state.persistence.lock().await;
    let event: AuditEvent = persistence.get_audit_event(event_id)?;
    drop(persistence);

    let response: AuditEventResponse = audit_event_to_response(&event);

    Ok(Json(response))
}

/// Handler for GET `/bootstrap/status` endpoint.
///
/// Returns a comprehensive bootstrap status summary.
async fn handle_get_bootstrap_status(
    AxumState(app_state): AxumState<AppState>,
) -> Result<Json<BootstrapStatusResponse>, HttpError> {
    info!("Handling get_bootstrap_status request");

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let area_counts: Vec<(u16, usize)> = persistence.count_areas_by_bid_year()?;
    let user_counts_by_year: Vec<(u16, usize)> = persistence.count_users_by_bid_year()?;
    let user_counts_by_area: Vec<(u16, String, usize)> =
        persistence.count_users_by_bid_year_and_area()?;
    drop(persistence);

    let response: BootstrapStatusResponse = get_bootstrap_status(
        &metadata,
        &area_counts,
        &user_counts_by_year,
        &user_counts_by_area,
    )?;

    Ok(Json(response))
}

/// Handler for POST `/auth/login` endpoint.
///
/// Authenticates an operator and creates a session.
async fn handle_login(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<zab_bid_api::LoginRequest>,
) -> Result<Json<zab_bid_api::LoginResponse>, HttpError> {
    info!(login_name = %req.login_name, "Handling login request");

    let mut persistence = app_state.persistence.lock().await;
    let response = zab_bid_api::login(&mut persistence, &req)?;
    drop(persistence);

    info!(
        login_name = %response.login_name,
        role = %response.role,
        "Login successful"
    );

    Ok(Json(response))
}

/// Handler for POST `/auth/logout` endpoint.
///
/// Deletes the current session.
async fn handle_logout(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(_actor, _operator): session::SessionOperator,
    Json(req): Json<LogoutRequest>,
) -> Result<StatusCode, HttpError> {
    info!("Handling logout request");

    let mut persistence = app_state.persistence.lock().await;
    zab_bid_api::logout(&mut persistence, &req.session_token)?;
    drop(persistence);

    info!("Logout successful");
    Ok(StatusCode::NO_CONTENT)
}

/// Handler for GET `/auth/me` endpoint.
///
/// Returns information about the currently authenticated operator with global capabilities.
async fn handle_whoami(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
) -> Result<Json<zab_bid_api::WhoAmIResponse>, HttpError> {
    info!(login_name = %operator.login_name, "Handling whoami request");

    let mut persistence = app_state.persistence.lock().await;
    let response = zab_bid_api::whoami(&mut persistence, &actor, &operator)?;
    drop(persistence);

    Ok(Json(response))
}

/// Handler for GET `/operators` endpoint.
///
/// Lists all operators with per-operator capabilities (admin only).
async fn handle_list_operators(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
) -> Result<Json<zab_bid_api::ListOperatorsResponse>, HttpError> {
    info!(actor_login = ?actor, "Handling list operators request");

    let mut persistence = app_state.persistence.lock().await;
    let response = zab_bid_api::list_operators(&mut persistence, &actor, &operator)?;
    drop(persistence);

    Ok(Json(response))
}

/// Handler for POST `/operators` endpoint.
///
/// Creates a new operator (admin only).
async fn handle_create_operator(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<CreateOperatorApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        new_operator_login = %req.login_name,
        "Handling create operator request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    let create_request: zab_bid_api::CreateOperatorRequest = zab_bid_api::CreateOperatorRequest {
        login_name: req.login_name.clone(),
        display_name: req.display_name.clone(),
        role: req.role.clone(),
        password: req.password.clone(),
        password_confirmation: req.password_confirmation.clone(),
    };

    let mut persistence = app_state.persistence.lock().await;
    let response =
        zab_bid_api::create_operator(&mut persistence, create_request, &actor, &operator, cause)?;
    drop(persistence);

    info!(
        operator_id = response.operator_id,
        login_name = %response.login_name,
        "Successfully created operator"
    );

    Ok(Json(WriteResponse {
        success: true,
        message: Some(format!("Created operator {}", req.login_name)),
        event_id: None,
    }))
}

/// Handler for POST `/operators/disable` endpoint.
///
/// Disables an operator (admin only).
async fn handle_disable_operator(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<DisableOperatorApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        target_operator_id = req.operator_id,
        "Handling disable operator request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    let disable_request: zab_bid_api::DisableOperatorRequest =
        zab_bid_api::DisableOperatorRequest {
            operator_id: req.operator_id,
        };

    let mut persistence = app_state.persistence.lock().await;
    let response =
        zab_bid_api::disable_operator(&mut persistence, disable_request, &actor, &operator, cause)?;
    drop(persistence);

    info!(
        operator_id = req.operator_id,
        "Successfully disabled operator"
    );

    Ok(Json(WriteResponse {
        success: true,
        message: Some(response.message),
        event_id: None,
    }))
}

/// Handler for POST `/operators/enable` endpoint.
///
/// Re-enables a disabled operator (admin only).
async fn handle_enable_operator(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<EnableOperatorApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        target_operator_id = req.operator_id,
        "Handling enable operator request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    let enable_request: zab_bid_api::EnableOperatorRequest = zab_bid_api::EnableOperatorRequest {
        operator_id: req.operator_id,
    };

    let mut persistence = app_state.persistence.lock().await;
    let response =
        zab_bid_api::enable_operator(&mut persistence, enable_request, &actor, &operator, cause)?;
    drop(persistence);

    info!(
        operator_id = req.operator_id,
        "Successfully re-enabled operator"
    );

    Ok(Json(WriteResponse {
        success: true,
        message: Some(response.message),
        event_id: None,
    }))
}

/// Handler for POST `/operators/delete` endpoint.
///
/// Deletes an operator (admin only, only if not referenced by audit events).
async fn handle_delete_operator(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<DeleteOperatorApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        target_operator_id = req.operator_id,
        "Handling delete operator request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    let delete_request: zab_bid_api::DeleteOperatorRequest = zab_bid_api::DeleteOperatorRequest {
        operator_id: req.operator_id,
    };

    let mut persistence = app_state.persistence.lock().await;
    let response =
        zab_bid_api::delete_operator(&mut persistence, delete_request, &actor, &operator, cause)?;
    drop(persistence);

    info!(
        operator_id = req.operator_id,
        "Successfully deleted operator"
    );

    Ok(Json(WriteResponse {
        success: true,
        message: Some(response.message),
        event_id: None,
    }))
}

/// Request body for create operator endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct CreateOperatorApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The operator login name.
    login_name: String,
    /// The operator display name.
    display_name: String,
    /// The operator role (Admin or Bidder).
    role: String,
    /// The operator password.
    password: String,
    /// The password confirmation.
    password_confirmation: String,
}

/// Request body for disable operator endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct DisableOperatorApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The operator ID to disable.
    operator_id: i64,
}

/// Request body for enable operator endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnableOperatorApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The operator ID to enable.
    operator_id: i64,
}

/// Request body for delete operator endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct DeleteOperatorApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The operator ID to delete.
    operator_id: i64,
}

/// Request body for set active bid year endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SetActiveBidYearApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical bid year identifier to set as active.
    bid_year_id: i64,
}

/// Request body for set expected area count endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SetExpectedAreaCountApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The expected number of areas.
    expected_count: u32,
}

/// Request body for set expected user count endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SetExpectedUserCountApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The canonical area identifier.
    area_id: i64,
    /// The expected number of users.
    expected_count: u32,
}

/// Request body for update user endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpdateUserApiRequest {
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The user's canonical internal identifier.
    user_id: i64,
    /// The user's initials.
    initials: String,
    /// The user's name.
    name: String,
    /// The canonical area identifier.
    area_id: i64,
    /// The user's type classification (CPC, CPC-IT, Dev-R, Dev-D).
    user_type: String,
    /// The user's crew number (1-7, optional).
    crew: Option<u8>,
    /// Cumulative NATCA bargaining unit date (ISO 8601).
    cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date (ISO 8601).
    natca_bu_date: String,
    /// Entry on Duty / FAA date (ISO 8601).
    eod_faa_date: String,
    /// Service Computation Date (ISO 8601).
    service_computation_date: String,
    /// Optional lottery value.
    lottery_value: Option<u32>,
}

/// API request to preview CSV user data.
#[derive(Debug, serde::Deserialize)]
struct PreviewCsvUsersApiRequest {
    /// The raw CSV content.
    csv_content: String,
}

/// API request to import selected CSV rows.
#[derive(Debug, serde::Deserialize)]
struct ImportCsvUsersApiRequest {
    /// The raw CSV content.
    csv_content: String,
    /// The row indices (0-based, excluding header) to import.
    selected_row_indices: Vec<usize>,
}

/// API request to override a user's area assignment.
#[derive(Debug, serde::Deserialize)]
struct OverrideAreaAssignmentApiRequest {
    /// The user's canonical identifier.
    user_id: i64,
    /// The new area ID to assign.
    new_area_id: i64,
    /// The reason for the override (min 10 characters).
    reason: String,
}

/// Request body for logout endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct LogoutRequest {
    /// The session token to delete.
    session_token: String,
}

/// Handler for GET `/auth/bootstrap/status` endpoint.
///
/// Checks if the system is in bootstrap mode (no operators exist).
async fn handle_bootstrap_status(
    AxumState(app_state): AxumState<AppState>,
) -> Result<Json<zab_bid_api::BootstrapAuthStatusResponse>, HttpError> {
    info!("Handling bootstrap status check");

    let mut persistence = app_state.persistence.lock().await;
    let response = zab_bid_api::check_bootstrap_status(&mut persistence)?;
    drop(persistence);

    Ok(Json(response))
}

/// Handler for POST `/auth/bootstrap/login` endpoint.
///
/// Performs bootstrap login with hardcoded admin/admin credentials.
async fn handle_bootstrap_login(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<zab_bid_api::BootstrapLoginRequest>,
) -> Result<Json<zab_bid_api::BootstrapLoginResponse>, HttpError> {
    info!("Handling bootstrap login request");

    let mut persistence = app_state.persistence.lock().await;
    let response = zab_bid_api::bootstrap_login(&mut persistence, &req)?;
    drop(persistence);

    info!("Bootstrap login successful");
    Ok(Json(response))
}

/// Handler for POST `/auth/bootstrap/create-first-admin` endpoint.
///
/// Creates the first admin operator during bootstrap.
async fn handle_create_first_admin(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<zab_bid_api::CreateFirstAdminRequest>,
) -> Result<Json<zab_bid_api::CreateFirstAdminResponse>, HttpError> {
    info!(login_name = %req.login_name, "Handling create first admin request");

    let mut persistence = app_state.persistence.lock().await;
    let response = zab_bid_api::create_first_admin(&mut persistence, req)?;
    drop(persistence);

    info!("First admin created successfully");
    Ok(Json(response))
}

/// Handler for POST `/bootstrap/bid-years/active` endpoint.
///
/// Sets the active bid year. Admin only.
async fn handle_set_active_bid_year(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<SetActiveBidYearApiRequest>,
) -> Result<Json<SetActiveBidYearResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        "Handling set_active_bid_year request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    drop(persistence);

    // Build API request
    let set_request: SetActiveBidYearRequest = SetActiveBidYearRequest {
        bid_year_id: req.bid_year_id,
    };

    // Execute command via API
    let mut persistence = app_state.persistence.lock().await;
    let response: SetActiveBidYearResponse = set_active_bid_year(
        &mut persistence,
        &metadata,
        &set_request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        year = response.year,
        bid_year_id = response.bid_year_id,
        "Successfully set active bid year"
    );

    // Broadcast live event
    app_state
        .live_events
        .broadcast(&LiveEvent::BidYearActivated {
            year: response.year,
        });

    Ok(Json(response))
}

/// Handler for POST `/lifecycle/bootstrap-complete` endpoint.
///
/// Transitions a bid year from `Draft` to `BootstrapComplete`.
async fn handle_transition_to_bootstrap_complete(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<TransitionToBootstrapCompleteApiRequest>,
) -> Result<Json<TransitionToBootstrapCompleteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        "Handling transition_to_bootstrap_complete request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let request: TransitionToBootstrapCompleteRequest = TransitionToBootstrapCompleteRequest {
        bid_year_id: req.bid_year_id,
    };

    let response: TransitionToBootstrapCompleteResponse = transition_to_bootstrap_complete(
        &mut persistence,
        &metadata,
        &request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        year = response.year,
        lifecycle_state = %response.lifecycle_state,
        "Successfully transitioned to BootstrapComplete"
    );

    Ok(Json(response))
}

/// Handler for POST `/lifecycle/canonicalized` endpoint.
///
/// Transitions a bid year from `BootstrapComplete` to `Canonicalized`.
async fn handle_transition_to_canonicalized(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<TransitionToCanonicalizedApiRequest>,
) -> Result<Json<TransitionToCanonicalizedResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        "Handling transition_to_canonicalized request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let request: TransitionToCanonicalizedRequest = TransitionToCanonicalizedRequest {
        bid_year_id: req.bid_year_id,
    };

    let response: TransitionToCanonicalizedResponse = transition_to_canonicalized(
        &mut persistence,
        &metadata,
        &request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        year = response.year,
        lifecycle_state = %response.lifecycle_state,
        "Successfully transitioned to Canonicalized"
    );

    Ok(Json(response))
}

/// Handler for POST `/lifecycle/bidding-active` endpoint.
///
/// Transitions a bid year from `Canonicalized` to `BiddingActive`.
async fn handle_transition_to_bidding_active(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<TransitionToBiddingActiveApiRequest>,
) -> Result<Json<TransitionToBiddingActiveResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        "Handling transition_to_bidding_active request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let request: TransitionToBiddingActiveRequest = TransitionToBiddingActiveRequest {
        bid_year_id: req.bid_year_id,
    };

    let response: TransitionToBiddingActiveResponse = transition_to_bidding_active(
        &mut persistence,
        &metadata,
        &request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        year = response.year,
        lifecycle_state = %response.lifecycle_state,
        "Successfully transitioned to BiddingActive"
    );

    Ok(Json(response))
}

/// Handler for POST `/lifecycle/bidding-closed` endpoint.
///
/// Transitions a bid year from `BiddingActive` to `BiddingClosed`.
async fn handle_transition_to_bidding_closed(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<TransitionToBiddingClosedApiRequest>,
) -> Result<Json<TransitionToBiddingClosedResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        "Handling transition_to_bidding_closed request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let request: TransitionToBiddingClosedRequest = TransitionToBiddingClosedRequest {
        bid_year_id: req.bid_year_id,
    };

    let response: TransitionToBiddingClosedResponse = transition_to_bidding_closed(
        &mut persistence,
        &metadata,
        &request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        year = response.year,
        lifecycle_state = %response.lifecycle_state,
        "Successfully transitioned to BiddingClosed"
    );

    Ok(Json(response))
}

/// Handler for POST `/bid-years/metadata` endpoint.
///
/// Updates the metadata (label and notes) for a bid year. Admin only.
async fn handle_update_bid_year_metadata(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<UpdateBidYearMetadataApiRequest>,
) -> Result<Json<UpdateBidYearMetadataResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        "Handling update_bid_year_metadata request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let request: UpdateBidYearMetadataRequest = UpdateBidYearMetadataRequest {
        bid_year_id: req.bid_year_id,
        label: req.label,
        notes: req.notes,
    };

    let response: UpdateBidYearMetadataResponse = update_bid_year_metadata(
        &mut persistence,
        &metadata,
        &request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        year = response.year,
        "Successfully updated bid year metadata"
    );

    Ok(Json(response))
}

/// Handler for GET `/bootstrap/bid-years/active` endpoint.
///
/// Gets the currently active bid year.
async fn handle_get_active_bid_year(
    AxumState(app_state): AxumState<AppState>,
) -> Result<Json<GetActiveBidYearResponse>, HttpError> {
    info!("Handling get_active_bid_year request");

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let response: GetActiveBidYearResponse = get_active_bid_year(&mut persistence, &metadata)?;
    drop(persistence);

    Ok(Json(response))
}

/// Handler for POST `/bootstrap/bid-years/{year}/expected-areas` endpoint.
///
/// Sets the expected area count for a bid year. Admin only.
async fn handle_set_expected_area_count(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<SetExpectedAreaCountApiRequest>,
) -> Result<Json<SetExpectedAreaCountResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        expected_count = req.expected_count,
        "Handling set_expected_area_count request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    drop(persistence);

    // Build API request
    let set_request: SetExpectedAreaCountRequest = SetExpectedAreaCountRequest {
        expected_count: req.expected_count,
    };

    // Execute command via API
    let mut persistence = app_state.persistence.lock().await;
    let response: SetExpectedAreaCountResponse = set_expected_area_count(
        &mut persistence,
        &metadata,
        &set_request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        expected_count = req.expected_count,
        "Successfully set expected area count"
    );

    Ok(Json(response))
}

/// Handler for POST `/bootstrap/areas/expected-users` endpoint.
///
/// Sets the expected user count for an area. Admin only.
async fn handle_set_expected_user_count(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<SetExpectedUserCountApiRequest>,
) -> Result<Json<SetExpectedUserCountResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = req.area_id,
        expected_count = req.expected_count,
        "Handling set_expected_user_count request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    drop(persistence);

    // Build API request
    let set_request: SetExpectedUserCountRequest = SetExpectedUserCountRequest {
        area_id: req.area_id,
        expected_count: req.expected_count,
    };

    // Execute command via API
    let mut persistence = app_state.persistence.lock().await;
    let response: SetExpectedUserCountResponse = set_expected_user_count(
        &mut persistence,
        &metadata,
        &set_request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        area_id = response.area_id,
        area = %response.area_code,
        expected_count = req.expected_count,
        "Successfully set expected user count"
    );

    Ok(Json(response))
}

/// Handler for PUT `/api/areas/update` endpoint.
///
/// Updates area metadata (display name). Admin only.
async fn handle_update_area(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<UpdateAreaRequest>,
) -> Result<Json<UpdateAreaResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = req.area_id,
        ?req.area_name,
        "Handling update_area request"
    );

    // Get current bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Execute command via API
    let response: UpdateAreaResponse =
        update_area(&mut persistence, &metadata, &req, &actor, &operator)?;
    drop(persistence);

    info!(
        area_id = response.area_id,
        area_code = %response.area_code,
        ?response.area_name,
        "Successfully updated area metadata"
    );

    Ok(Json(response))
}

/// Handler for PUT `/users/{initials}` endpoint.
///
/// Updates an existing user. Admin only.
async fn handle_update_user(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<UpdateUserApiRequest>,
) -> Result<Json<UpdateUserResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        area_id = req.area_id,
        initials = %req.initials,
        "Handling update_user request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Get the user's current area from the database (not the target area)
    let current_area_id: i64 =
        persistence
            .get_user_area_id(req.user_id)
            .map_err(|e| HttpError {
                status: StatusCode::NOT_FOUND,
                message: format!("User not found: {e}"),
            })?;

    // Resolve the user's current area from metadata to get bid_year and area
    let (bid_year_ref, current_area_ref) = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(current_area_id))
        .map(|(by, a)| (by.clone(), a.clone()))
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("User's current area (ID {current_area_id}) not found in metadata"),
        })?;

    // Load state from the user's CURRENT area (where they are now)
    let state: State = persistence.get_current_state(&bid_year_ref, &current_area_ref)?;

    // Also resolve the target area (where the user is moving to) for validation
    let target_area_ref = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(req.area_id))
        .map(|(_, a)| a.clone())
        .ok_or_else(|| HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Target area with ID {} not found", req.area_id),
        })?;

    // Build API request (bid_year no longer needed in request)
    let update_request: UpdateUserRequest = UpdateUserRequest {
        user_id: req.user_id,
        initials: req.initials.clone(),
        name: req.name.clone(),
        area_id: req.area_id,
        user_type: req.user_type.clone(),
        crew: req.crew,
        cumulative_natca_bu_date: req.cumulative_natca_bu_date.clone(),
        natca_bu_date: req.natca_bu_date.clone(),
        eod_faa_date: req.eod_faa_date.clone(),
        service_computation_date: req.service_computation_date.clone(),
        lottery_value: req.lottery_value,
    };

    // Execute command via API (persistence already locked)
    let result: ApiResult<UpdateUserResponse> = update_user(
        &mut persistence,
        &metadata,
        &state,
        &update_request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        initials = %req.initials,
        event_id = result.audit_event.event_id,
        "Successfully updated user"
    );

    // Broadcast live event
    let bid_year_for_event = result
        .audit_event
        .bid_year
        .as_ref()
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("UpdateUser event missing bid year"),
        })?;

    app_state.live_events.broadcast(&LiveEvent::UserUpdated {
        bid_year: bid_year_for_event.year(),
        area: target_area_ref.area_code().to_string(),
        initials: req.initials.clone(),
    });

    Ok(Json(result.response))
}

/// Handler for GET `/bootstrap/completeness` endpoint.
///
/// Gets the bootstrap completeness status for all bid years and areas.
async fn handle_get_bootstrap_completeness(
    AxumState(app_state): AxumState<AppState>,
) -> Result<Json<GetBootstrapCompletenessResponse>, HttpError> {
    info!("Handling get_bootstrap_completeness request");

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let response: GetBootstrapCompletenessResponse =
        get_bootstrap_completeness(&mut persistence, &metadata)?;
    drop(persistence);

    Ok(Json(response))
}

/// Handler for POST `/bootstrap/users/csv/preview` endpoint.
///
/// Previews and validates CSV user data without persisting.
async fn handle_preview_csv_users(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<PreviewCsvUsersApiRequest>,
) -> Result<Json<PreviewCsvUsersResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        "Handling preview_csv_users request"
    );

    // Get bootstrap metadata
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Build API request
    let preview_request: PreviewCsvUsersRequest = PreviewCsvUsersRequest {
        csv_content: req.csv_content,
    };

    // Execute preview via API (no persistence mutations)
    let response: PreviewCsvUsersResponse =
        preview_csv_users(&metadata, &mut persistence, &preview_request, &actor)?;

    drop(persistence);

    info!(
        total_rows = response.total_rows,
        valid_count = response.valid_count,
        invalid_count = response.invalid_count,
        "Successfully previewed CSV users"
    );

    Ok(Json(response))
}

/// Handler for POST `/bootstrap/users/csv/import` endpoint.
///
/// Imports selected CSV rows as users.
async fn handle_import_csv_users(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<ImportCsvUsersApiRequest>,
) -> Result<Json<ImportCsvUsersResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        selected_count = req.selected_row_indices.len(),
        "Handling import_csv_users request"
    );

    let cause: Cause = Cause::new(
        String::from("csv_import"),
        String::from("Bulk user import from CSV"),
    );

    // Get bootstrap metadata and current state
    // Note: CSV import may span multiple areas, so we use a dummy state
    // The actual state will be loaded per-user during import
    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Resolve active bid year from metadata
    let active_year: u16 = persistence.get_active_bid_year().map_err(|e| HttpError {
        status: StatusCode::BAD_REQUEST,
        message: format!("Failed to get active bid year: {e}"),
    })?;

    let bid_year: BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == active_year)
        .cloned()
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Active year {active_year} not found in metadata"),
        })?;

    // We need a state instance for the import handler signature
    // Use the first area if available, or create a dummy one
    let state: State = if let Some((by, first_area)) = metadata.areas.first() {
        persistence
            .get_current_state(by, first_area)
            .unwrap_or_else(|_| State::new(by.clone(), first_area.clone()))
    } else {
        // Fallback: create dummy area (should not happen in practice)
        State::new(bid_year, Area::new("DUMMY"))
    };

    // Build API request (bid_year no longer needed in request)
    let import_request = ImportCsvUsersRequest {
        csv_content: req.csv_content,
        selected_row_indices: req.selected_row_indices,
    };

    // Execute import via API (persistence already locked)
    let response = import_csv_users(
        &metadata,
        &state,
        &mut persistence,
        &import_request,
        &actor,
        &operator,
        &cause,
    )?;

    drop(persistence);

    info!(
        total_selected = response.total_selected,
        successful_count = response.successful_count,
        failed_count = response.failed_count,
        "Successfully completed CSV import"
    );

    // Broadcast live events for successful imports
    for result in &response.results {
        if result.status == CsvImportRowStatus::Success
            && let Some(ref initials) = result.initials
        {
            app_state.live_events.broadcast(&LiveEvent::UserRegistered {
                bid_year: response.bid_year,
                area: String::from("MULTI"), // CSV can span areas
                initials: initials.clone(),
            });
        }
    }

    Ok(Json(response))
}

/// Handler for POST `/users/override-area` endpoint.
///
/// Overrides a user's area assignment after canonicalization.
async fn handle_override_area_assignment(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<OverrideAreaAssignmentApiRequest>,
) -> Result<Json<OverrideAreaAssignmentResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        user_id = req.user_id,
        new_area_id = req.new_area_id,
        "Handling override_area_assignment request"
    );

    let mut persistence = app_state.persistence.lock().await;

    // Build API request
    let override_request = OverrideAreaAssignmentRequest {
        user_id: req.user_id,
        new_area_id: req.new_area_id,
        reason: req.reason.clone(),
    };

    // Execute override via API
    let response =
        override_area_assignment(&mut persistence, &override_request, &actor, &operator)?;

    drop(persistence);

    info!(
        user_id = req.user_id,
        audit_event_id = response.audit_event_id,
        "Successfully overrode area assignment"
    );

    // Note: We could broadcast a user_updated event here, but the override
    // is a special case that may need its own event type in the future

    Ok(Json(response))
}

/// Handler for GET `/bid-status/area` endpoint.
///
/// Gets bid status for all users in an area across all rounds.
async fn handle_get_bid_status_for_area(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Query(query): Query<GetBidStatusForAreaQuery>,
) -> Result<Json<zab_bid_api::GetBidStatusForAreaResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        bid_year_id = query.bid_year_id,
        area_id = query.area_id,
        "Handling get_bid_status_for_area request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let request = zab_bid_api::GetBidStatusForAreaRequest {
        bid_year_id: query.bid_year_id,
        area_id: query.area_id,
    };

    let response = zab_bid_api::get_bid_status_for_area(&mut persistence, &request, &actor)?;
    drop(persistence);

    Ok(Json(response))
}

/// Handler for GET `/bid-status/user-round` endpoint.
///
/// Gets bid status for a specific user in a specific round with history.
async fn handle_get_bid_status(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Query(query): Query<GetBidStatusQuery>,
) -> Result<Json<zab_bid_api::GetBidStatusResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        user_id = query.user_id,
        round_id = query.round_id,
        "Handling get_bid_status request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let request = zab_bid_api::GetBidStatusRequest {
        bid_year_id: query.bid_year_id,
        area_id: query.area_id,
        user_id: query.user_id,
        round_id: query.round_id,
    };

    let response = zab_bid_api::get_bid_status(&mut persistence, &request, &actor)?;
    drop(persistence);

    Ok(Json(response))
}

/// Handler for POST `/bid-status/transition` endpoint.
///
/// Transitions a user's bid status for a round.
async fn handle_transition_bid_status(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<TransitionBidStatusApiRequest>,
) -> Result<Json<zab_bid_api::TransitionBidStatusResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_status_id = req.bid_status_id,
        new_status = %req.new_status,
        "Handling transition_bid_status request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let request = zab_bid_api::TransitionBidStatusRequest {
        bid_status_id: req.bid_status_id,
        new_status: req.new_status.clone(),
        notes: req.notes.clone(),
    };

    let response =
        zab_bid_api::transition_bid_status(&mut persistence, &request, &actor, &operator)?;
    drop(persistence);

    info!(
        bid_status_id = req.bid_status_id,
        new_status = %response.new_status,
        "Successfully transitioned bid status"
    );

    Ok(Json(response))
}

/// Handler for POST `/bid-status/bulk-update` endpoint.
///
/// Bulk updates bid status for multiple users in a round.
async fn handle_bulk_update_bid_status(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<BulkUpdateBidStatusApiRequest>,
) -> Result<Json<zab_bid_api::BulkUpdateBidStatusResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        user_count = req.user_ids.len(),
        round_id = req.round_id,
        new_status = %req.new_status,
        "Handling bulk_update_bid_status request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let request = zab_bid_api::BulkUpdateBidStatusRequest {
        bid_year_id: req.bid_year_id,
        area_id: req.area_id,
        round_id: req.round_id,
        user_ids: req.user_ids.clone(),
        new_status: req.new_status.clone(),
        notes: req.notes.clone(),
    };

    let response =
        zab_bid_api::bulk_update_bid_status(&mut persistence, &request, &actor, &operator)?;
    drop(persistence);

    info!(
        updated_count = response.updated_count,
        "Successfully bulk updated bid status"
    );

    Ok(Json(response))
}

#[derive(Debug, serde::Deserialize)]
struct GetBidStatusForAreaQuery {
    bid_year_id: i64,
    area_id: i64,
}

#[derive(Debug, serde::Deserialize)]
#[allow(clippy::struct_field_names)]
struct GetBidStatusQuery {
    bid_year_id: i64,
    area_id: i64,
    user_id: i64,
    round_id: i64,
}

#[derive(Debug, serde::Deserialize)]
struct TransitionBidStatusApiRequest {
    bid_status_id: i64,
    new_status: String,
    notes: String,
}

#[derive(Debug, serde::Deserialize)]
struct BulkUpdateBidStatusApiRequest {
    bid_year_id: i64,
    area_id: i64,
    round_id: i64,
    user_ids: Vec<i64>,
    new_status: String,
    notes: String,
}

/// Request for updating user participation flags (Phase 29A)
#[derive(serde::Deserialize)]
struct UpdateUserParticipationApiRequest {
    user_id: i64,
    excluded_from_bidding: bool,
    excluded_from_leave_calculation: bool,
}

/// Request for setting bid schedule (Phase 29C)
#[derive(serde::Deserialize)]
struct SetBidScheduleApiRequest {
    cause_id: String,
    cause_description: String,
    bid_year_id: i64,
    timezone: String,
    start_date: String,
    window_start_time: String,
    window_end_time: String,
    bidders_per_day: i32,
}

/// Path parameter for getting bid schedule (Phase 29C)
#[derive(serde::Deserialize)]
struct BidYearIdPath {
    bid_year_id: i64,
}

/// Request for adjusting bid order (Phase 29G)
#[derive(serde::Deserialize)]
struct AdjustBidOrderApiRequest {
    bid_year_id: i64,
    area_id: i64,
    adjustments: Vec<BidOrderAdjustmentItem>,
    reason: String,
}

#[derive(serde::Deserialize)]
struct BidOrderAdjustmentItem {
    user_id: i64,
    new_order: i64,
}

/// Request for adjusting bid window (Phase 29G)
#[derive(serde::Deserialize)]
struct AdjustBidWindowApiRequest {
    bid_year_id: i64,
    area_id: i64,
    user_id: i64,
    round_id: i64,
    new_window_start: String,
    new_window_end: String,
    reason: String,
}

/// Request for recalculating bid windows (Phase 29G)
#[derive(serde::Deserialize)]
struct RecalculateBidWindowsApiRequest {
    bid_year_id: i64,
    area_id: i64,
    user_ids: Vec<i64>,
    rounds: Vec<i64>,
    reason: String,
}

/// Request for creating a round group (Phase 29B)
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct CreateRoundGroupApiRequest {
    cause_id: String,
    cause_description: String,
    bid_year_id: i64,
    name: String,
    editing_enabled: bool,
}

/// Query for listing round groups (Phase 29B)
#[derive(serde::Deserialize)]
struct ListRoundGroupsQuery {
    bid_year_id: i64,
}

/// Request for updating a round group (Phase 29B)
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct UpdateRoundGroupApiRequest {
    cause_id: String,
    cause_description: String,
    name: String,
    editing_enabled: bool,
}

/// Request for deleting a round group (Phase 29B)
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct DeleteRoundGroupApiRequest {
    cause_id: String,
    cause_description: String,
}

/// Request for creating a round (Phase 29B)
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct CreateRoundApiRequest {
    cause_id: String,
    cause_description: String,
    round_group_id: i64,
    round_number: u32,
    name: String,
    slots_per_day: u32,
    max_groups: u32,
    max_total_hours: u32,
    include_holidays: bool,
    allow_overbid: bool,
}

/// Query for listing rounds (Phase 29B)
#[derive(serde::Deserialize)]
struct ListRoundsQuery {
    round_group_id: i64,
}

/// Request for updating a round (Phase 29B)
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct UpdateRoundApiRequest {
    cause_id: String,
    cause_description: String,
    round_group_id: i64,
    round_number: u32,
    name: String,
    slots_per_day: u32,
    max_groups: u32,
    max_total_hours: u32,
    include_holidays: bool,
    allow_overbid: bool,
}

/// Request for deleting a round (Phase 29B)
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct DeleteRoundApiRequest {
    cause_id: String,
    cause_description: String,
}

/// Request for reviewing a No Bid user (Phase 29D)
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ReviewNoBidUserApiRequest {
    cause_id: String,
    cause_description: String,
}

/// Query for getting bid order preview (Phase 29D)
#[derive(serde::Deserialize)]
struct GetBidOrderPreviewQuery {
    bid_year_id: i64,
    area_id: i64,
}

/// Request for confirming ready to bid (Phase 29E)
#[derive(serde::Deserialize)]
struct ConfirmReadyToBidApiRequest {
    cause_id: String,
    cause_description: String,
    bid_year_id: i64,
    confirmation: String,
}

/// Request for overriding user eligibility
#[derive(serde::Deserialize)]
struct OverrideEligibilityApiRequest {
    user_id: i64,
    can_bid: bool,
    reason: String,
}

/// Request for overriding user bid order
#[derive(serde::Deserialize)]
struct OverrideBidOrderApiRequest {
    user_id: i64,
    bid_order: Option<i32>,
    reason: String,
}

/// Request for overriding user bid window
#[derive(serde::Deserialize)]
struct OverrideBidWindowApiRequest {
    user_id: i64,
    window_start: Option<String>,
    window_end: Option<String>,
    reason: String,
}

/// Handler for POST `/users/participation` endpoint (Phase 29A).
///
/// Updates user participation flags. Admin only.
async fn handle_update_user_participation(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<UpdateUserParticipationApiRequest>,
) -> Result<Json<UpdateUserParticipationResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        user_id = req.user_id,
        excluded_from_bidding = req.excluded_from_bidding,
        excluded_from_leave_calculation = req.excluded_from_leave_calculation,
        "Handling update_user_participation request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Get the active bid year
    let active_year: u16 = persistence.get_active_bid_year().map_err(|e| HttpError {
        status: StatusCode::BAD_REQUEST,
        message: format!("Failed to get active bid year: {e}"),
    })?;

    // Find the bid year in metadata to get bid_year_id
    let bid_year_id = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == active_year)
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Active bid year not found in metadata".to_string(),
        })?;

    // Get lifecycle state
    let lifecycle_state: BidYearLifecycle = persistence
        .get_lifecycle_state(bid_year_id)
        .map_err(|e| HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to get lifecycle state: {e}"),
        })
        .and_then(|s| {
            s.parse::<BidYearLifecycle>().map_err(|_| HttpError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Invalid lifecycle state".to_string(),
            })
        })?;

    // Build API request
    let api_request = UpdateUserParticipationRequest {
        user_id: req.user_id,
        excluded_from_bidding: req.excluded_from_bidding,
        excluded_from_leave_calculation: req.excluded_from_leave_calculation,
    };

    // Convert AuthenticatedActor to Actor
    let audit_actor = zab_bid_audit::Actor::with_operator(
        operator.login_name.clone(),
        "operator".to_string(),
        operator.operator_id,
        operator.login_name.clone(),
        operator.display_name.clone(),
    );

    // Execute command via API
    let response = update_user_participation(
        &metadata,
        &mut persistence,
        &api_request,
        &audit_actor,
        lifecycle_state,
    )?;

    drop(persistence);

    info!(
        user_id = response.user_id,
        bid_year_id = response.bid_year_id,
        "Successfully updated user participation flags"
    );

    Ok(Json(response))
}

/// Handler for POST `/bid-schedule` endpoint (Phase 29C).
///
/// Sets the bid schedule for a bid year. Admin only.
async fn handle_set_bid_schedule(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<SetBidScheduleApiRequest>,
) -> Result<Json<SetBidScheduleResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        timezone = %req.timezone,
        start_date = %req.start_date,
        "Handling set_bid_schedule request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Build API request
    let api_request = SetBidScheduleRequest {
        bid_year_id: req.bid_year_id,
        timezone: req.timezone,
        start_date: req.start_date,
        window_start_time: req.window_start_time,
        window_end_time: req.window_end_time,
        bidders_per_day: req.bidders_per_day.try_into().map_err(|_| HttpError {
            status: StatusCode::BAD_REQUEST,
            message: "bidders_per_day must be non-negative".to_string(),
        })?,
    };

    // Execute command via API
    let response = set_bid_schedule(
        &mut persistence,
        &metadata,
        &api_request,
        &actor,
        &operator,
        cause,
    )?;

    drop(persistence);

    info!(
        bid_year_id = response.bid_year_id,
        "Successfully set bid schedule"
    );

    Ok(Json(response))
}

/// Handler for GET `/bid-schedule/{bid_year_id}` endpoint (Phase 29C).
///
/// Retrieves the bid schedule for a bid year.
async fn handle_get_bid_schedule(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Path(path): axum::extract::Path<BidYearIdPath>,
) -> Result<Json<GetBidScheduleResponse>, HttpError> {
    info!(
        bid_year_id = path.bid_year_id,
        "Handling get_bid_schedule request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Execute query via API
    let response = get_bid_schedule(&mut persistence, &metadata, path.bid_year_id)?;

    drop(persistence);

    Ok(Json(response))
}

/// Handler for POST `/bid-order/adjust` endpoint (Phase 29G).
///
/// Adjusts bid order for multiple users. Admin only.
async fn handle_adjust_bid_order(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<AdjustBidOrderApiRequest>,
) -> Result<Json<AdjustBidOrderResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        area_id = req.area_id,
        num_adjustments = req.adjustments.len(),
        "Handling adjust_bid_order request"
    );

    let mut persistence = app_state.persistence.lock().await;

    // Convert adjustments to API format
    let adjustments: Vec<BidOrderAdjustment> = req
        .adjustments
        .into_iter()
        .map(|adj| BidOrderAdjustment {
            user_id: adj.user_id,
            new_bid_order: adj.new_order.try_into().map_or(0, |v: i32| v),
        })
        .collect();

    // Build API request
    let api_request = AdjustBidOrderRequest {
        adjustments,
        reason: req.reason,
    };

    // Execute command via API
    let response = adjust_bid_order(
        &mut persistence,
        req.bid_year_id,
        req.area_id,
        &api_request,
        &actor,
        &operator,
    )?;

    drop(persistence);

    info!(
        users_adjusted = response.users_adjusted,
        audit_event_id = response.audit_event_id,
        "Successfully adjusted bid order"
    );

    Ok(Json(response))
}

/// Handler for POST `/bid-windows/adjust` endpoint (Phase 29G).
///
/// Adjusts a bid window for a specific user and round. Admin only.
async fn handle_adjust_bid_window(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<AdjustBidWindowApiRequest>,
) -> Result<Json<AdjustBidWindowResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        area_id = req.area_id,
        user_id = req.user_id,
        round_id = req.round_id,
        "Handling adjust_bid_window request"
    );

    let mut persistence = app_state.persistence.lock().await;

    // Build API request
    let api_request = AdjustBidWindowRequest {
        user_id: req.user_id,
        round_id: req.round_id,
        new_window_start: req.new_window_start,
        new_window_end: req.new_window_end,
        reason: req.reason,
    };

    // Execute command via API
    let response = adjust_bid_window(
        &mut persistence,
        req.bid_year_id,
        req.area_id,
        &api_request,
        &actor,
        &operator,
    )?;

    drop(persistence);

    info!(
        audit_event_id = response.audit_event_id,
        "Successfully adjusted bid window"
    );

    Ok(Json(response))
}

/// Handler for POST `/bid-windows/recalculate` endpoint (Phase 29G).
///
/// Recalculates bid windows for specified users and rounds. Admin only.
async fn handle_recalculate_bid_windows(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<RecalculateBidWindowsApiRequest>,
) -> Result<Json<RecalculateBidWindowsResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        area_id = req.area_id,
        num_users = req.user_ids.len(),
        num_rounds = req.rounds.len(),
        "Handling recalculate_bid_windows request"
    );

    let mut persistence = app_state.persistence.lock().await;

    // Build API request
    let api_request = RecalculateBidWindowsRequest {
        user_ids: req.user_ids,
        rounds: req.rounds,
        reason: req.reason,
    };

    // Execute command via API
    let response = recalculate_bid_windows(
        &mut persistence,
        req.bid_year_id,
        req.area_id,
        &api_request,
        &actor,
        &operator,
    )?;

    drop(persistence);

    info!(
        windows_recalculated = response.windows_recalculated,
        audit_event_id = response.audit_event_id,
        "Successfully recalculated bid windows"
    );

    Ok(Json(response))
}

/// Handler for POST `/api/round-groups` endpoint.
///
/// Creates a new round group. Admin only.
async fn handle_create_round_group(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Json(req): Json<CreateRoundGroupApiRequest>,
) -> Result<Json<CreateRoundGroupResponse>, HttpError> {
    info!(
        bid_year_id = req.bid_year_id,
        name = %req.name,
        "Handling create_round_group request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let request: CreateRoundGroupRequest = CreateRoundGroupRequest {
        name: req.name,
        editing_enabled: req.editing_enabled,
    };

    let response: CreateRoundGroupResponse =
        create_round_group(&mut persistence, req.bid_year_id, &request, &actor)?;
    drop(persistence);

    info!(
        round_group_id = response.round_group_id,
        "Successfully created round group"
    );

    Ok(Json(response))
}

/// Handler for GET `/api/round-groups` endpoint.
///
/// Lists all round groups for a bid year. Authenticated.
async fn handle_list_round_groups(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Query(query): Query<ListRoundGroupsQuery>,
) -> Result<Json<ListRoundGroupsResponse>, HttpError> {
    info!(
        bid_year_id = query.bid_year_id,
        "Handling list_round_groups request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let response: ListRoundGroupsResponse =
        list_round_groups(&mut persistence, query.bid_year_id, &actor)?;
    drop(persistence);

    info!(
        round_group_count = response.round_groups.len(),
        "Successfully listed round groups"
    );

    Ok(Json(response))
}

/// Handler for PUT `/api/round-groups/{id}` endpoint.
///
/// Updates a round group. Admin only.
async fn handle_update_round_group(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Path(round_group_id): Path<i64>,
    Json(req): Json<UpdateRoundGroupApiRequest>,
) -> Result<Json<UpdateRoundGroupResponse>, HttpError> {
    info!(
        round_group_id = round_group_id,
        "Handling update_round_group request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let request: UpdateRoundGroupRequest = UpdateRoundGroupRequest {
        round_group_id,
        name: req.name,
        editing_enabled: req.editing_enabled,
    };

    let response: UpdateRoundGroupResponse =
        update_round_group(&mut persistence, &request, &actor)?;
    drop(persistence);

    info!(
        round_group_id = response.round_group_id,
        "Successfully updated round group"
    );

    Ok(Json(response))
}

/// Handler for DELETE `/api/round-groups/{id}` endpoint.
///
/// Deletes a round group. Admin only.
async fn handle_delete_round_group(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Path(round_group_id): Path<i64>,
    Json(_req): Json<DeleteRoundGroupApiRequest>,
) -> Result<Json<DeleteRoundGroupResponse>, HttpError> {
    info!(
        round_group_id = round_group_id,
        "Handling delete_round_group request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let response: DeleteRoundGroupResponse =
        delete_round_group(&mut persistence, round_group_id, &actor)?;
    drop(persistence);

    info!(
        round_group_id = round_group_id,
        "Successfully deleted round group"
    );

    Ok(Json(response))
}

/// Handler for POST `/api/rounds` endpoint.
///
/// Creates a new round. Admin only.
async fn handle_create_round(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Json(req): Json<CreateRoundApiRequest>,
) -> Result<Json<CreateRoundResponse>, HttpError> {
    info!(
        round_group_id = req.round_group_id,
        "Handling create_round request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let request: CreateRoundRequest = CreateRoundRequest {
        round_group_id: req.round_group_id,
        round_number: req.round_number,
        name: req.name,
        slots_per_day: req.slots_per_day,
        max_groups: req.max_groups,
        max_total_hours: req.max_total_hours,
        include_holidays: req.include_holidays,
        allow_overbid: req.allow_overbid,
    };

    let response: CreateRoundResponse =
        create_round(&mut persistence, req.round_group_id, &request, &actor)?;
    drop(persistence);

    info!(round_id = response.round_id, "Successfully created round");

    Ok(Json(response))
}

/// Handler for GET `/api/rounds` endpoint.
///
/// Lists all rounds for a round group. Authenticated.
async fn handle_list_rounds(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Query(query): Query<ListRoundsQuery>,
) -> Result<Json<ListRoundsResponse>, HttpError> {
    info!(
        round_group_id = query.round_group_id,
        "Handling list_rounds request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let response: ListRoundsResponse = list_rounds(&mut persistence, query.round_group_id, &actor)?;
    drop(persistence);

    info!(
        round_count = response.rounds.len(),
        "Successfully listed rounds"
    );

    Ok(Json(response))
}

/// Handler for PUT `/api/rounds/{id}` endpoint.
///
/// Updates a round. Admin only.
async fn handle_update_round(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Path(round_id): Path<i64>,
    Json(req): Json<UpdateRoundApiRequest>,
) -> Result<Json<UpdateRoundResponse>, HttpError> {
    info!(round_id = round_id, "Handling update_round request");

    let mut persistence = app_state.persistence.lock().await;

    let request: UpdateRoundRequest = UpdateRoundRequest {
        round_id,
        round_group_id: req.round_group_id,
        round_number: req.round_number,
        name: req.name,
        slots_per_day: req.slots_per_day,
        max_groups: req.max_groups,
        max_total_hours: req.max_total_hours,
        include_holidays: req.include_holidays,
        allow_overbid: req.allow_overbid,
    };

    let response: UpdateRoundResponse = update_round(&mut persistence, &request, &actor)?;
    drop(persistence);

    info!(round_id = response.round_id, "Successfully updated round");

    Ok(Json(response))
}

/// Handler for DELETE `/api/rounds/{id}` endpoint.
///
/// Deletes a round. Admin only.
async fn handle_delete_round(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Path(round_id): Path<i64>,
    Json(_req): Json<DeleteRoundApiRequest>,
) -> Result<Json<DeleteRoundResponse>, HttpError> {
    info!(round_id = round_id, "Handling delete_round request");

    let mut persistence = app_state.persistence.lock().await;

    let response: DeleteRoundResponse = delete_round(&mut persistence, round_id, &actor)?;
    drop(persistence);

    info!(round_id = round_id, "Successfully deleted round");

    Ok(Json(response))
}

/// Handler for GET `/api/readiness/{bid_year_id}` endpoint.
///
/// Gets readiness evaluation for a bid year. Admin only.
async fn handle_get_bid_year_readiness(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(_actor, _operator): session::SessionOperator,
    Path(bid_year_id): Path<i64>,
) -> Result<Json<GetBidYearReadinessResponse>, HttpError> {
    info!(
        bid_year_id = bid_year_id,
        "Handling get_bid_year_readiness request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let response: GetBidYearReadinessResponse =
        get_bid_year_readiness(&mut persistence, &metadata, bid_year_id)?;
    drop(persistence);

    info!(
        is_ready = response.is_ready,
        blocking_count = response.blocking_reasons.len(),
        "Successfully evaluated readiness"
    );

    Ok(Json(response))
}

/// Handler for POST `/api/users/{user_id}/review-no-bid` endpoint.
///
/// Marks a No Bid user as reviewed. Admin only.
async fn handle_review_no_bid_user(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, _operator): session::SessionOperator,
    Path(user_id): Path<i64>,
    Json(_req): Json<ReviewNoBidUserApiRequest>,
) -> Result<Json<ReviewNoBidUserResponse>, HttpError> {
    info!(user_id = user_id, "Handling review_no_bid_user request");

    let mut persistence = app_state.persistence.lock().await;

    let response: ReviewNoBidUserResponse = review_no_bid_user(&mut persistence, user_id, &actor)?;
    drop(persistence);

    info!(
        user_id = response.user_id,
        "Successfully reviewed No Bid user"
    );

    Ok(Json(response))
}

/// Handler for GET `/api/bid-order/preview` endpoint.
///
/// Previews bid order without persisting. Authenticated.
async fn handle_get_bid_order_preview(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(_actor, _operator): session::SessionOperator,
    Query(query): Query<GetBidOrderPreviewQuery>,
) -> Result<Json<GetBidOrderPreviewResponse>, HttpError> {
    info!(
        bid_year_id = query.bid_year_id,
        area_id = query.area_id,
        "Handling get_bid_order_preview request"
    );

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let response: GetBidOrderPreviewResponse = get_bid_order_preview(
        &mut persistence,
        &metadata,
        query.bid_year_id,
        query.area_id,
    )?;
    drop(persistence);

    info!(
        position_count = response.positions.len(),
        "Successfully generated bid order preview"
    );

    Ok(Json(response))
}

/// Handler for POST `/api/confirm-ready-to-bid` endpoint.
///
/// Confirms readiness and enters bidding phase. Admin only. IRREVERSIBLE.
async fn handle_confirm_ready_to_bid(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<ConfirmReadyToBidApiRequest>,
) -> Result<Json<ConfirmReadyToBidResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year_id = req.bid_year_id,
        "Handling confirm_ready_to_bid request (IRREVERSIBLE)"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    let mut persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    let request: ConfirmReadyToBidRequest = ConfirmReadyToBidRequest {
        bid_year_id: req.bid_year_id,
        confirmation: req.confirmation,
    };

    let response: ConfirmReadyToBidResponse = confirm_ready_to_bid(
        &mut persistence,
        &metadata,
        &request,
        &actor,
        &operator,
        cause,
    )?;
    drop(persistence);

    info!(
        bid_year_id = response.bid_year_id,
        "Successfully confirmed ready to bid - SYSTEM NOW IN BIDDING PHASE"
    );

    Ok(Json(response))
}

/// Handler for POST `/api/users/override-eligibility` endpoint.
///
/// Overrides user eligibility status. Admin only.
async fn handle_override_eligibility(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<OverrideEligibilityApiRequest>,
) -> Result<Json<OverrideEligibilityResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        user_id = req.user_id,
        "Handling override_eligibility request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let request: OverrideEligibilityRequest = OverrideEligibilityRequest {
        user_id: req.user_id,
        can_bid: req.can_bid,
        reason: req.reason,
    };

    let response: OverrideEligibilityResponse =
        override_eligibility(&mut persistence, &request, &actor, &operator)?;
    drop(persistence);

    info!(
        audit_event_id = response.audit_event_id,
        "Successfully overrode user eligibility"
    );

    Ok(Json(response))
}

/// Handler for POST `/api/users/override-bid-order` endpoint.
///
/// Overrides user bid order position. Admin only.
async fn handle_override_bid_order(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<OverrideBidOrderApiRequest>,
) -> Result<Json<OverrideBidOrderResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        user_id = req.user_id,
        "Handling override_bid_order request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let request: OverrideBidOrderRequest = OverrideBidOrderRequest {
        user_id: req.user_id,
        bid_order: req.bid_order,
        reason: req.reason,
    };

    let response: OverrideBidOrderResponse =
        override_bid_order(&mut persistence, &request, &actor, &operator)?;
    drop(persistence);

    info!(
        audit_event_id = response.audit_event_id,
        "Successfully overrode user bid order"
    );

    Ok(Json(response))
}

/// Handler for POST `/api/users/override-bid-window` endpoint.
///
/// Overrides user bid window dates. Admin only.
async fn handle_override_bid_window(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<OverrideBidWindowApiRequest>,
) -> Result<Json<OverrideBidWindowResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        user_id = req.user_id,
        "Handling override_bid_window request"
    );

    let mut persistence = app_state.persistence.lock().await;

    let request: OverrideBidWindowRequest = OverrideBidWindowRequest {
        user_id: req.user_id,
        window_start: req.window_start,
        window_end: req.window_end,
        reason: req.reason,
    };

    let response: OverrideBidWindowResponse =
        override_bid_window(&mut persistence, &request, &actor, &operator)?;
    drop(persistence);

    info!(
        audit_event_id = response.audit_event_id,
        "Successfully overrode user bid window"
    );

    Ok(Json(response))
}

/// Health check endpoint for Docker and load balancers
async fn handle_health() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "healthy\n")
}

#[allow(clippy::too_many_lines)]
fn build_router(state: AppState) -> Router {
    let live_broadcaster = Arc::clone(&state.live_events);

    let api_router = Router::new()
        // Health check endpoint (no authentication required)
        .route("/health", get(handle_health))
        // Bootstrap authentication endpoints (no authentication required)
        .route("/auth/bootstrap/status", get(handle_bootstrap_status))
        .route("/auth/bootstrap/login", post(handle_bootstrap_login))
        .route(
            "/auth/bootstrap/create-first-admin",
            post(handle_create_first_admin),
        )
        // Authentication endpoints (no authentication required)
        .route("/auth/login", post(handle_login))
        // State-changing endpoints (authentication required)
        .route("/bid_years", post(handle_create_bid_year))
        .route("/areas", post(handle_create_area))
        .route("/users", post(handle_register_user))
        .route("/checkpoint", post(handle_checkpoint))
        .route("/finalize", post(handle_finalize))
        .route("/rollback", post(handle_rollback))
        // Authenticated read endpoints
        .route("/auth/logout", post(handle_logout))
        .route("/auth/me", get(handle_whoami))
        // Operator management endpoints (admin only)
        .route("/operators", get(handle_list_operators))
        .route("/operators", post(handle_create_operator))
        .route("/operators/disable", post(handle_disable_operator))
        .route("/operators/enable", post(handle_enable_operator))
        .route("/operators/delete", post(handle_delete_operator))
        // Read-only endpoints (no authentication required for now)
        .route("/bid_years", get(handle_list_bid_years))
        .route("/areas", get(handle_list_areas))
        .route("/users", get(handle_list_users))
        .route("/leave/availability", get(handle_get_leave_availability))
        .route("/state/current", get(handle_get_current_state))
        .route("/state/historical", get(handle_get_historical_state))
        .route("/audit/timeline", get(handle_get_audit_timeline))
        .route("/audit/event/{id}", get(handle_get_audit_event))
        .route("/bootstrap/status", get(handle_get_bootstrap_status))
        // Bootstrap completeness endpoints
        .route(
            "/bootstrap/bid-years/active",
            post(handle_set_active_bid_year),
        )
        .route(
            "/bootstrap/bid-years/active",
            get(handle_get_active_bid_year),
        )
        .route(
            "/bootstrap/bid-years/expected-areas",
            post(handle_set_expected_area_count),
        )
        .route(
            "/bootstrap/areas/expected-users",
            post(handle_set_expected_user_count),
        )
        .route("/areas/update", post(handle_update_area))
        .route(
            "/bootstrap/completeness",
            get(handle_get_bootstrap_completeness),
        )
        // Lifecycle transition endpoints (admin only)
        .route(
            "/lifecycle/bootstrap-complete",
            post(handle_transition_to_bootstrap_complete),
        )
        .route(
            "/lifecycle/canonicalized",
            post(handle_transition_to_canonicalized),
        )
        .route(
            "/lifecycle/bidding-active",
            post(handle_transition_to_bidding_active),
        )
        .route(
            "/lifecycle/bidding-closed",
            post(handle_transition_to_bidding_closed),
        )
        .route("/bid-years/metadata", post(handle_update_bid_year_metadata))
        .route("/users/update", post(handle_update_user))
        .route(
            "/users/override-area",
            post(handle_override_area_assignment),
        )
        .route(
            "/bootstrap/users/csv/preview",
            post(handle_preview_csv_users),
        )
        .route("/bootstrap/users/csv/import", post(handle_import_csv_users))
        // Bid status endpoints
        .route("/bid-status/area", get(handle_get_bid_status_for_area))
        .route("/bid-status/user-round", get(handle_get_bid_status))
        .route("/bid-status/transition", post(handle_transition_bid_status))
        .route(
            "/bid-status/bulk-update",
            post(handle_bulk_update_bid_status),
        )
        // Phase 29A: User participation flags
        .route(
            "/users/participation",
            post(handle_update_user_participation),
        )
        // Phase 29C: Bid schedule
        .route("/bid-schedule", post(handle_set_bid_schedule))
        .route("/bid-schedule/{bid_year_id}", get(handle_get_bid_schedule))
        // Phase 29G: Post-confirmation bid order adjustments
        .route("/bid-order/adjust", post(handle_adjust_bid_order))
        .route("/bid-windows/adjust", post(handle_adjust_bid_window))
        .route(
            "/bid-windows/recalculate",
            post(handle_recalculate_bid_windows),
        )
        // Phase 29B: Round management
        .route("/round-groups", post(handle_create_round_group))
        .route("/round-groups", get(handle_list_round_groups))
        .route("/round-groups/{id}", post(handle_update_round_group))
        .route("/round-groups/{id}", delete(handle_delete_round_group))
        .route("/rounds", post(handle_create_round))
        .route("/rounds", get(handle_list_rounds))
        .route("/rounds/{id}", post(handle_update_round))
        .route("/rounds/{id}", delete(handle_delete_round))
        // Phase 29D: Readiness evaluation
        .route(
            "/readiness/{bid_year_id}",
            get(handle_get_bid_year_readiness),
        )
        .route(
            "/users/{user_id}/review-no-bid",
            post(handle_review_no_bid_user),
        )
        .route("/bid-order/preview", get(handle_get_bid_order_preview))
        // Phase 29E: Confirmation (IRREVERSIBLE)
        .route("/confirm-ready-to-bid", post(handle_confirm_ready_to_bid))
        // Override endpoints
        .route(
            "/users/override-eligibility",
            post(handle_override_eligibility),
        )
        .route("/users/override-bid-order", post(handle_override_bid_order))
        .route(
            "/users/override-bid-window",
            post(handle_override_bid_window),
        )
        .with_state(state);

    let live_router = Router::new()
        .route("/live", axum::routing::get(live::live_events_handler))
        .with_state(live_broadcaster);

    Router::new()
        .nest("/api", api_router)
        .nest("/api", live_router)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args: Args = Args::parse();

    // Validate argument combinations
    args.validate()
        .map_err(|e| format!("Invalid arguments: {e}"))?;

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Initializing ZAB Bid Server");
    info!("Selected database backend: {}", args.db_backend);

    // Initialize persistence based on selected backend
    let persistence: Persistence = match args.db_backend.as_str() {
        "sqlite" => {
            if let Some(db_path) = &args.database {
                info!("Using SQLite file-based database at: {}", db_path);
                Persistence::new_with_file(db_path)?
            } else {
                info!("Using SQLite in-memory database");
                Persistence::new_in_memory()?
            }
        }
        "mysql" => {
            let database_url = args
                .database_url
                .as_ref()
                .ok_or("MySQL backend requires --database-url")?;
            info!("Using MySQL database at: {}", database_url);
            Persistence::new_with_mysql(database_url)?
        }
        _ => {
            // This should never be reached due to validation, but handle defensively
            return Err(format!("Unsupported backend: {}", args.db_backend).into());
        }
    };

    let app_state: AppState = AppState {
        persistence: Arc::new(Mutex::new(persistence)),
        live_events: Arc::new(LiveEventBroadcaster::new()),
    };

    // Build router
    let app: Router = build_router(app_state);

    // Bind to address
    let addr: std::net::SocketAddr = format!("0.0.0.0:{}", args.port).parse()?;
    info!("Server listening on {}", addr);

    // Run server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode as HttpStatusCode},
    };
    use tower::ServiceExt;

    /// Helper to create test app state with in-memory persistence.
    fn create_test_app_state() -> AppState {
        let persistence: Persistence =
            Persistence::new_in_memory().expect("Failed to create in-memory persistence");
        AppState {
            persistence: Arc::new(Mutex::new(persistence)),
            live_events: Arc::new(LiveEventBroadcaster::new()),
        }
    }

    // ========================================================================
    // Phase 24G Argument Validation Tests
    // ========================================================================

    #[test]
    fn test_args_default_sqlite_backend() {
        let args = Args {
            db_backend: String::from("sqlite"),
            database: None,
            database_url: None,
            port: 3000,
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_args_sqlite_with_file() {
        let args = Args {
            db_backend: String::from("sqlite"),
            database: Some(String::from("./test.db")),
            database_url: None,
            port: 3000,
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_args_sqlite_rejects_database_url() {
        let args = Args {
            db_backend: String::from("sqlite"),
            database: None,
            database_url: Some(String::from("mysql://localhost/test")),
            port: 3000,
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("SQLite backend does not support --database-url")
        );
    }

    #[test]
    fn test_args_mysql_requires_database_url() {
        let args = Args {
            db_backend: String::from("mysql"),
            database: None,
            database_url: None,
            port: 3000,
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("MySQL backend requires --database-url")
        );
    }

    #[test]
    fn test_args_mysql_with_database_url() {
        let args = Args {
            db_backend: String::from("mysql"),
            database: None,
            database_url: Some(String::from("mysql://user:pass@localhost/zabbid")),
            port: 3000,
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_args_mysql_rejects_database_flag() {
        let args = Args {
            db_backend: String::from("mysql"),
            database: Some(String::from("./test.db")),
            database_url: Some(String::from("mysql://localhost/test")),
            port: 3000,
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("MySQL backend does not support --database")
        );
    }

    #[test]
    fn test_args_unknown_backend_rejected() {
        let args = Args {
            db_backend: String::from("postgres"),
            database: None,
            database_url: None,
            port: 3000,
        };
        let result = args.validate();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Unknown database backend"));
        assert!(error_msg.contains("postgres"));
    }

    #[test]
    fn test_args_sqlite_with_both_flags_rejected() {
        // SQLite with database_url should fail
        let args = Args {
            db_backend: String::from("sqlite"),
            database: Some(String::from("./test.db")),
            database_url: Some(String::from("mysql://localhost/test")),
            port: 3000,
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("SQLite backend does not support --database-url")
        );
    }

    // ========================================================================
    // Phase 14b Authentication Tests
    // ========================================================================

    /// Helper to create an operator and get a session token.
    async fn create_operator_and_login(
        app_state: &AppState,
        login_name: &str,
        display_name: &str,
        role: &str,
    ) -> String {
        {
            let mut persistence = app_state.persistence.lock().await;
            persistence
                .create_operator(login_name, display_name, "password", role)
                .expect("Failed to create operator");
            drop(persistence);
        }

        let mut persistence = app_state.persistence.lock().await;
        let login_req = zab_bid_api::LoginRequest {
            login_name: login_name.to_string(),
            password: String::from("password"),
        };
        let response = zab_bid_api::login(&mut persistence, &login_req).expect("Failed to login");
        drop(persistence);
        response.session_token
    }

    #[tokio::test]
    async fn test_login_creates_session() {
        let app_state = create_test_app_state();
        let app = build_router(app_state.clone());

        // Create operator
        let mut persistence = app_state.persistence.lock().await;
        persistence
            .create_operator("admin1", "Admin User", "password", "Admin")
            .expect("Failed to create operator");
        drop(persistence);

        // Login
        let login_req = zab_bid_api::LoginRequest {
            login_name: String::from("admin1"),
            password: String::from("password"),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&login_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let login_response: zab_bid_api::LoginResponse =
            serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(login_response.login_name.to_lowercase(), "admin1");
        assert_eq!(login_response.role, "Admin");
        assert!(!login_response.session_token.is_empty());
    }

    #[tokio::test]
    async fn test_unauthenticated_write_request_rejected() {
        let app_state = create_test_app_state();
        let app = build_router(app_state);

        let req = CreateBidYearApiRequest {
            cause_id: String::from("test"),
            cause_description: String::from("Test"),
            year: 2026,
            start_date: String::from("2026-01-05"),
            num_pay_periods: 26,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bid_years")
                    .header("content-type", "application/json")
                    // No Authorization header
                    .body(Body::from(serde_json::to_string(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_session_token_rejected() {
        let app_state = create_test_app_state();
        let app = build_router(app_state);

        let req = CreateBidYearApiRequest {
            cause_id: String::from("test"),
            cause_description: String::from("Test"),
            year: 2026,
            start_date: String::from("2026-01-05"),
            num_pay_periods: 26,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bid_years")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer invalid_token")
                    .body(Body::from(serde_json::to_string(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_bidder_cannot_create_bid_year() {
        let app_state = create_test_app_state();
        let app = build_router(app_state.clone());

        let bidder_token =
            create_operator_and_login(&app_state, "bidder1", "Bidder User", "Bidder").await;

        let req = CreateBidYearApiRequest {
            cause_id: String::from("test"),
            cause_description: String::from("Test"),
            year: 2026,
            start_date: String::from("2026-01-05"),
            num_pay_periods: 26,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bid_years")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {bidder_token}"))
                    .body(Body::from(serde_json::to_string(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_disabled_operator_cannot_login() {
        let app_state = create_test_app_state();

        // Create and disable operator
        {
            let mut persistence = app_state.persistence.lock().await;
            let operator_id = persistence
                .create_operator("admin1", "Admin User", "password", "Admin")
                .expect("Failed to create operator");
            persistence
                .disable_operator(operator_id)
                .expect("Failed to disable operator");
        }

        // Try to login
        let result = {
            let mut persistence = app_state.persistence.lock().await;
            let login_req = zab_bid_api::LoginRequest {
                login_name: String::from("admin1"),
                password: String::from("password"),
            };
            zab_bid_api::login(&mut persistence, &login_req)
        };

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_authorization_failure_does_not_create_audit_event() {
        let app_state = create_test_app_state();
        let app = build_router(app_state.clone());

        let bidder_token =
            create_operator_and_login(&app_state, "bidder1", "Bidder User", "Bidder").await;

        // Try to create bid year as bidder (should fail)
        let req = CreateBidYearApiRequest {
            cause_id: String::from("test"),
            cause_description: String::from("Test"),
            year: 2026,
            start_date: String::from("2026-01-05"),
            num_pay_periods: 26,
        };

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bid_years")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {bidder_token}"))
                    .body(Body::from(serde_json::to_string(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);

        // Verify no audit events were created
        let metadata = app_state
            .persistence
            .lock()
            .await
            .get_bootstrap_metadata()
            .unwrap();
        assert_eq!(metadata.bid_years.len(), 0);
    }
}
