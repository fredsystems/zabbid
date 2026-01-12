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
#![allow(clippy::multiple_crate_versions)]

mod live;
mod session;

use axum::{
    Json, Router,
    extract::{Path, Query, State as AxumState},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use live::{LiveEvent, LiveEventBroadcaster};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_api::{
    ApiError, ApiResult, BootstrapStatusResponse, CreateAreaRequest, CreateBidYearRequest,
    GetLeaveAvailabilityResponse, ListAreasRequest, ListAreasResponse, ListBidYearsResponse,
    ListUsersResponse, RegisterUserRequest, RegisterUserResponse, checkpoint, create_area,
    create_bid_year, finalize, get_bootstrap_status, get_current_state, get_historical_state,
    get_leave_availability, list_areas, list_bid_years, list_users, register_user, rollback,
};
use zab_bid_audit::{AuditEvent, Cause};
use zab_bid_domain::{Area, BidYear, CanonicalBidYear, Initials};
use zab_bid_persistence::{PersistenceError, SqlitePersistence};

/// ZAB Bid Server - HTTP server for the ZAB Bidding System
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the `SQLite` database file. If not provided, uses in-memory database.
    #[arg(short, long)]
    database: Option<String>,

    /// Port to bind the server to
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

/// Application state shared across handlers.
///
/// This contains the persistence layer wrapped in a Mutex to allow
/// safe concurrent access, and a live event broadcaster for WebSocket streaming.
#[derive(Clone)]
struct AppState {
    /// The persistence layer for audit events and state snapshots.
    persistence: Arc<Mutex<SqlitePersistence>>,
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
    /// The bid year (e.g., 2026).
    bid_year: u16,
    /// The user's initials.
    initials: String,
    /// The user's name.
    name: String,
    /// The user's area identifier.
    area: String,
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
    /// The bid year scope.
    bid_year: u16,
    /// The area scope.
    area: String,
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
    /// The bid year.
    bid_year: u16,
    /// The area identifier.
    area_id: String,
}

/// Query parameters for listing areas.
#[derive(Debug, Deserialize)]
struct ListAreasQuery {
    /// The bid year.
    bid_year: u16,
}

/// Query parameters for listing users.
#[derive(Debug, Deserialize)]
struct ListUsersQuery {
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
}

/// Bid year information for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BidYearInfoApiResponse {
    /// The year value.
    year: u16,
    /// The start date (ISO 8601).
    start_date: String,
    /// The number of pay periods.
    num_pay_periods: u8,
    /// The end date (ISO 8601).
    end_date: String,
    /// The number of areas in this bid year.
    area_count: usize,
    /// The total number of users across all areas in this bid year.
    total_user_count: usize,
}

/// API response for listing bid years.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListBidYearsApiResponse {
    /// The list of bid years with canonical metadata.
    bid_years: Vec<BidYearInfoApiResponse>,
}

/// API response for listing areas.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListAreasApiResponse {
    /// The bid year.
    bid_year: u16,
    /// The list of areas with metadata.
    areas: Vec<AreaInfoResponse>,
}

/// Area information response for API.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AreaInfoResponse {
    /// The area identifier.
    area_id: String,
    /// The number of users in this area.
    user_count: usize,
}

/// API response for listing users.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListUsersApiResponse {
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
    /// The list of users.
    users: Vec<UserInfoResponse>,
}

/// User information for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserInfoResponse {
    /// The user's initials.
    initials: String,
    /// The user's name.
    name: String,
    /// The user's crew (optional).
    crew: Option<u8>,
    /// The user's type classification (CPC, CPC-IT, Dev-R, Dev-D).
    user_type: String,
    /// Total hours earned (from Phase 9, post-rounding).
    earned_hours: u16,
    /// Total days earned.
    earned_days: u16,
    /// Remaining hours available (may be negative if overdrawn).
    remaining_hours: i32,
    /// Remaining days available (may be negative if overdrawn).
    remaining_days: i32,
    /// Whether all leave has been exhausted.
    is_exhausted: bool,
    /// Whether leave balance is overdrawn.
    is_overdrawn: bool,
}

/// Query parameters for leave availability.
#[derive(Debug, Clone, Deserialize)]
struct LeaveAvailabilityQuery {
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
    /// The user's initials.
    initials: String,
}

/// API response for leave availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaveAvailabilityApiResponse {
    /// The bid year.
    bid_year: u16,
    /// The user's initials.
    initials: String,
    /// Total hours earned.
    earned_hours: u16,
    /// Total days earned.
    earned_days: u16,
    /// Total hours used.
    used_hours: u16,
    /// Remaining hours available.
    remaining_hours: i32,
    /// Remaining days available.
    remaining_days: i32,
    /// Whether all leave has been exhausted.
    is_exhausted: bool,
    /// Whether leave balance is overdrawn.
    is_overdrawn: bool,
    /// Human-readable explanation.
    explanation: String,
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

/// API response for register user operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisterUserApiResponse {
    /// Success indicator.
    success: bool,
    /// The bid year the user was registered for.
    bid_year: u16,
    /// The user's initials.
    initials: String,
    /// The user's name.
    name: String,
    /// A success message.
    message: String,
    /// The event ID of the persisted audit event.
    event_id: i64,
}

/// Query parameters for current state endpoint.
#[derive(Debug, Deserialize)]
struct CurrentStateQuery {
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
}

/// Query parameters for historical state endpoint.
#[derive(Debug, Deserialize)]
struct HistoricalStateQuery {
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
    /// The timestamp (ISO 8601 format).
    timestamp: String,
}

/// Query parameters for audit timeline endpoint.
#[derive(Debug, Deserialize)]
struct AuditTimelineQuery {
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
}

/// Serializable representation of State for JSON responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateResponse {
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
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
    /// The bid year.
    bid_year: u16,
    /// The area.
    area: String,
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
            ApiError::InvalidInput { .. } => Self {
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
fn state_to_response(state: &State) -> StateResponse {
    StateResponse {
        bid_year: state.bid_year.year(),
        area: state.area.id().to_string(),
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
    }
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
        bid_year: event.bid_year.year(),
        area: event.area.id().to_string(),
    }
}

/// Handler for POST `/bid_years` endpoint.
///
/// Creates a new bid year.
async fn handle_create_bid_year(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<CreateBidYearApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        year = req.year,
        "Handling create_bid_year request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let persistence = app_state.persistence.lock().await;
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
    drop(persistence);

    info!(
        event_id = event_id,
        year = req.year,
        "Successfully created bid year"
    );

    // Broadcast live event
    app_state
        .live_events
        .broadcast(&LiveEvent::BidYearCreated { year: req.year });

    Ok(Json(WriteResponse {
        success: true,
        message: Some(format!("Created bid year {}", req.year)),
        event_id: Some(event_id),
    }))
}

/// Handler for POST `/areas` endpoint.
///
/// Creates a new area within a bid year.
async fn handle_create_area(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<CreateAreaApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        bid_year = req.bid_year,
        area_id = %req.area_id,
        "Handling create_area request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    drop(persistence);

    // Build API request
    let create_request: CreateAreaRequest = CreateAreaRequest {
        bid_year: req.bid_year,
        area_id: req.area_id.clone(),
    };

    // Execute command via API
    let bootstrap_result: BootstrapResult =
        create_area(&metadata, &create_request, &actor, &operator, cause)?;

    // Persist the bootstrap result
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_bootstrap(&bootstrap_result)?;
    drop(persistence);

    info!(
        event_id = event_id,
        bid_year = req.bid_year,
        area_id = %req.area_id,
        "Successfully created area"
    );

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::AreaCreated {
        bid_year: req.bid_year,
        area: req.area_id.clone(),
    });

    Ok(Json(WriteResponse {
        success: true,
        message: Some(format!(
            "Created area '{}' in bid year {}",
            req.area_id, req.bid_year
        )),
        event_id: Some(event_id),
    }))
}

/// Handler for GET `/bid_years` endpoint.
///
/// Lists all bid years.
async fn handle_list_bid_years(
    AxumState(app_state): AxumState<AppState>,
) -> Result<Json<ListBidYearsApiResponse>, HttpError> {
    info!("Handling list_bid_years request");

    let persistence = app_state.persistence.lock().await;
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years()?;

    // Get aggregate counts
    let area_counts: Vec<(u16, usize)> = persistence.count_areas_by_bid_year()?;
    let user_counts: Vec<(u16, usize)> = persistence.count_users_by_bid_year()?;
    drop(persistence);

    let response: ListBidYearsResponse = list_bid_years(&canonical_bid_years)?;

    // Convert to API response format with ISO 8601 date strings and counts
    let api_bid_years: Vec<BidYearInfoApiResponse> = response
        .bid_years
        .iter()
        .map(|info| {
            let area_count: usize = area_counts
                .iter()
                .find(|(year, _)| *year == info.year)
                .map_or(0, |(_, count)| *count);
            let total_user_count: usize = user_counts
                .iter()
                .find(|(year, _)| *year == info.year)
                .map_or(0, |(_, count)| *count);
            BidYearInfoApiResponse {
                year: info.year,
                start_date: info.start_date.to_string(),
                num_pay_periods: info.num_pay_periods,
                end_date: info.end_date.to_string(),
                area_count,
                total_user_count,
            }
        })
        .collect();

    Ok(Json(ListBidYearsApiResponse {
        bid_years: api_bid_years,
    }))
}

/// Handler for GET `/areas` endpoint.
///
/// Lists all areas for a given bid year.
async fn handle_list_areas(
    AxumState(app_state): AxumState<AppState>,
    Query(query): Query<ListAreasQuery>,
) -> Result<Json<ListAreasApiResponse>, HttpError> {
    info!(bid_year = query.bid_year, "Handling list_areas request");

    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let bid_year: BidYear = BidYear::new(query.bid_year);

    // Get user counts per area
    let user_counts: Vec<(String, usize)> = persistence.count_users_by_area(&bid_year)?;
    drop(persistence);

    let request: ListAreasRequest = ListAreasRequest {
        bid_year: query.bid_year,
    };
    let response: ListAreasResponse = list_areas(&metadata, &request)?;

    // Build area info with user counts
    let areas_with_counts: Vec<AreaInfoResponse> = response
        .areas
        .into_iter()
        .map(|area_info| {
            let user_count: usize = user_counts
                .iter()
                .find(|(area_id, _)| area_id == &area_info.area_id)
                .map_or(0, |(_, count)| *count);
            AreaInfoResponse {
                area_id: area_info.area_id,
                user_count,
            }
        })
        .collect();

    Ok(Json(ListAreasApiResponse {
        bid_year: response.bid_year,
        areas: areas_with_counts,
    }))
}

/// Handler for GET `/users` endpoint.
///
/// Lists all users for a given bid year and area.
async fn handle_list_users(
    AxumState(app_state): AxumState<AppState>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersApiResponse>, HttpError> {
    info!(
        bid_year = query.bid_year,
        area = %query.area,
        "Handling list_users request"
    );

    let persistence = app_state.persistence.lock().await;
    let bid_year: BidYear = BidYear::new(query.bid_year);
    let area: Area = Area::new(&query.area);

    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let canonical_bid_years: Vec<CanonicalBidYear> = persistence.list_bid_years()?;
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    let response: ListUsersResponse =
        list_users(&metadata, &canonical_bid_years, &bid_year, &area, &state)?;

    let users: Vec<UserInfoResponse> = response
        .users
        .into_iter()
        .map(|u| UserInfoResponse {
            initials: u.initials,
            name: u.name,
            crew: u.crew,
            user_type: u.user_type,
            earned_hours: u.earned_hours,
            earned_days: u.earned_days,
            remaining_hours: u.remaining_hours,
            remaining_days: u.remaining_days,
            is_exhausted: u.is_exhausted,
            is_overdrawn: u.is_overdrawn,
        })
        .collect();

    Ok(Json(ListUsersApiResponse {
        bid_year: response.bid_year,
        area: response.area,
        users,
    }))
}

/// Handler for GET `/leave/availability` endpoint.
///
/// Returns leave availability for a specific user.
async fn handle_get_leave_availability(
    AxumState(app_state): AxumState<AppState>,
    Query(query): Query<LeaveAvailabilityQuery>,
) -> Result<Json<LeaveAvailabilityApiResponse>, HttpError> {
    info!(
        bid_year = query.bid_year,
        area = %query.area,
        initials = %query.initials,
        "Handling get_leave_availability request"
    );

    let persistence = app_state.persistence.lock().await;
    let bid_year: BidYear = BidYear::new(query.bid_year);
    let area: Area = Area::new(&query.area);
    let initials: Initials = Initials::new(&query.initials);

    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;

    // Get canonical bid year for accrual calculation
    let canonical_bid_years: Vec<CanonicalBidYear> = persistence.list_bid_years()?;
    let canonical_bid_year: &CanonicalBidYear = canonical_bid_years
        .iter()
        .find(|by| by.year() == query.bid_year)
        .ok_or_else(|| {
            PersistenceError::DatabaseError(format!("Bid year {} not found", query.bid_year))
        })?;

    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    let response: GetLeaveAvailabilityResponse =
        get_leave_availability(&metadata, canonical_bid_year, &area, &initials, &state)?;

    Ok(Json(LeaveAvailabilityApiResponse {
        bid_year: response.bid_year,
        initials: response.initials,
        earned_hours: response.earned_hours,
        earned_days: response.earned_days,
        used_hours: response.used_hours,
        remaining_hours: response.remaining_hours,
        remaining_days: response.remaining_days,
        is_exhausted: response.is_exhausted,
        is_overdrawn: response.is_overdrawn,
        explanation: response.explanation,
    }))
}

/// Handler for POST `/register_user` endpoint.
///
/// Authenticates the actor, authorizes the action, and registers a new user.
async fn handle_register_user(
    AxumState(app_state): AxumState<AppState>,
    session::SessionOperator(actor, operator): session::SessionOperator,
    Json(req): Json<RegisterUserApiRequest>,
) -> Result<Json<RegisterUserApiResponse>, HttpError> {
    info!(
        actor_login = %operator.login_name,
        role = ?actor.role,
        initials = %req.initials,
        "Handling register_user request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let bid_year: BidYear = BidYear::new(req.bid_year);
    let area: Area = Area::new(&req.area);
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    // Build API request
    let register_request: RegisterUserRequest = RegisterUserRequest {
        bid_year: req.bid_year,
        initials: req.initials,
        name: req.name,
        area: req.area.clone(),
        user_type: req.user_type,
        crew: req.crew,
        cumulative_natca_bu_date: req.cumulative_natca_bu_date,
        natca_bu_date: req.natca_bu_date,
        eod_faa_date: req.eod_faa_date,
        service_computation_date: req.service_computation_date,
        lottery_value: req.lottery_value,
    };

    // Execute command via API
    let result: ApiResult<RegisterUserResponse> = register_user(
        &metadata,
        &state,
        register_request,
        &actor,
        &operator,
        cause,
    )?;

    // Persist the transition
    let mut persistence = app_state.persistence.lock().await;
    let transition_result: TransitionResult = TransitionResult {
        audit_event: result.audit_event.clone(),
        new_state: result.new_state.clone(),
    };
    let event_id: i64 = persistence.persist_transition(&transition_result, false)?;
    drop(persistence);

    info!(
        event_id = event_id,
        initials = %result.response.initials,
        "Successfully registered user"
    );

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::UserRegistered {
        bid_year: result.response.bid_year,
        area: req.area.clone(),
        initials: result.response.initials.clone(),
    });

    Ok(Json(RegisterUserApiResponse {
        success: true,
        bid_year: result.response.bid_year,
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
        bid_year = req.bid_year,
        area = %req.area,
        "Handling checkpoint request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let bid_year: BidYear = BidYear::new(req.bid_year);
    let area: Area = Area::new(&req.area);
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    // Execute command via API
    let result: TransitionResult = checkpoint(&metadata, &state, &actor, &operator, cause)?;

    // Persist the transition
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_transition(
        &result,
        SqlitePersistence::should_snapshot(&result.audit_event.action.name),
    )?;
    drop(persistence);

    info!(event_id = event_id, "Successfully created checkpoint");

    // Broadcast live event
    app_state
        .live_events
        .broadcast(&LiveEvent::CheckpointCreated {
            bid_year: req.bid_year,
            area: req.area.clone(),
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
        bid_year = req.bid_year,
        area = %req.area,
        "Handling finalize request"
    );

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let bid_year: BidYear = BidYear::new(req.bid_year);
    let area: Area = Area::new(&req.area);
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    // Execute command via API
    let result: TransitionResult = finalize(&metadata, &state, &actor, &operator, cause)?;

    // Persist the transition
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_transition(
        &result,
        SqlitePersistence::should_snapshot(&result.audit_event.action.name),
    )?;
    drop(persistence);

    info!(event_id = event_id, "Successfully finalized round");

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::RoundFinalized {
        bid_year: req.bid_year,
        area: req.area.clone(),
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
        bid_year = req.bid_year,
        area = %req.area,
        target_event_id = ?req.target_event_id,
        "Handling rollback request"
    );

    let target_event_id: i64 = req.target_event_id.ok_or_else(|| HttpError {
        status: StatusCode::BAD_REQUEST,
        message: String::from("target_event_id is required for rollback"),
    })?;

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get bootstrap metadata and current state
    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let bid_year: BidYear = BidYear::new(req.bid_year);
    let area: Area = Area::new(&req.area);
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    // Execute command via API
    let result: TransitionResult =
        rollback(&metadata, &state, target_event_id, &actor, &operator, cause)?;

    // Persist the transition
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_transition(
        &result,
        SqlitePersistence::should_snapshot(&result.audit_event.action.name),
    )?;
    drop(persistence);

    info!(
        event_id = event_id,
        target_event_id = target_event_id,
        "Successfully rolled back to event"
    );

    // Broadcast live event
    app_state.live_events.broadcast(&LiveEvent::RolledBack {
        bid_year: req.bid_year,
        area: req.area.clone(),
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
        bid_year = params.bid_year,
        area = %params.area,
        "Handling get_current_state request"
    );

    let bid_year: BidYear = BidYear::new(params.bid_year);
    let area: Area = Area::new(&params.area);

    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    let validated_state: State = get_current_state(&metadata, &bid_year, &area, state)?;
    let response: StateResponse = state_to_response(&validated_state);

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
        bid_year = params.bid_year,
        area = %params.area,
        timestamp = %params.timestamp,
        "Handling get_historical_state request"
    );

    let bid_year: BidYear = BidYear::new(params.bid_year);
    let area: Area = Area::new(&params.area);

    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    let state: State = persistence.get_historical_state(&bid_year, &area, &params.timestamp)?;
    drop(persistence);

    let validated_state: State = get_historical_state(&metadata, &bid_year, &area, state)?;
    let response: StateResponse = state_to_response(&validated_state);

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
        bid_year = params.bid_year,
        area = %params.area,
        "Handling get_audit_timeline request"
    );

    let bid_year: BidYear = BidYear::new(params.bid_year);
    let area: Area = Area::new(&params.area);

    let persistence = app_state.persistence.lock().await;
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

    let persistence = app_state.persistence.lock().await;
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

    let persistence = app_state.persistence.lock().await;
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
/// Returns information about the currently authenticated operator.
async fn handle_whoami(
    session::SessionOperator(_actor, operator): session::SessionOperator,
) -> Result<Json<zab_bid_api::WhoAmIResponse>, HttpError> {
    info!(login_name = %operator.login_name, "Handling whoami request");

    let response = zab_bid_api::whoami(&operator);

    Ok(Json(response))
}

/// Request body for logout endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct LogoutRequest {
    /// The session token to delete.
    session_token: String,
}

/// Builds the application router with all endpoints.
fn build_router(state: AppState) -> Router {
    let live_broadcaster = Arc::clone(&state.live_events);

    let api_router = Router::new()
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

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Initializing ZAB Bid Server");

    // Initialize persistence (in-memory or file-based based on CLI argument)
    let persistence: SqlitePersistence = if let Some(db_path) = &args.database {
        info!("Using file-based database at: {}", db_path);
        SqlitePersistence::new_with_file(db_path)?
    } else {
        info!("Using in-memory database");
        SqlitePersistence::new_in_memory()?
    };

    let app_state: AppState = AppState {
        persistence: Arc::new(Mutex::new(persistence)),
        live_events: Arc::new(LiveEventBroadcaster::new()),
    };

    // Build router
    let app: Router = build_router(app_state);

    // Bind to address
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", args.port).parse()?;
    info!("Server listening on {}", addr);

    // Run server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode as HttpStatusCode},
    };
    use tower::ServiceExt;

    /// Helper to create test app state with in-memory persistence.
    fn create_test_app_state() -> AppState {
        let persistence: SqlitePersistence =
            SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");
        AppState {
            persistence: Arc::new(Mutex::new(persistence)),
            live_events: Arc::new(LiveEventBroadcaster::new()),
        }
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
                .create_operator(login_name, display_name, role)
                .expect("Failed to create operator");
            drop(persistence);
        }

        let mut persistence = app_state.persistence.lock().await;
        let login_req = zab_bid_api::LoginRequest {
            login_name: login_name.to_string(),
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
            .create_operator("admin1", "Admin User", "Admin")
            .expect("Failed to create operator");
        drop(persistence);

        // Login
        let login_req = zab_bid_api::LoginRequest {
            login_name: String::from("admin1"),
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
                .create_operator("admin1", "Admin User", "Admin")
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
