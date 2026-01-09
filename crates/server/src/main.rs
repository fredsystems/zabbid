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

use axum::{
    Json, Router,
    extract::{Path, Query, State as AxumState},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_api::{
    ApiError, ApiResult, AuthenticatedActor, CreateAreaRequest, CreateBidYearRequest,
    ListAreasRequest, ListAreasResponse, ListBidYearsResponse, ListUsersResponse,
    RegisterUserRequest, RegisterUserResponse, Role, authenticate_stub, checkpoint, create_area,
    create_bid_year, finalize, list_areas, list_bid_years, list_users, register_user, rollback,
};
use zab_bid_audit::{AuditEvent, Cause};
use zab_bid_domain::{Area, BidYear};
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
/// safe concurrent access.
#[derive(Clone)]
struct AppState {
    /// The persistence layer for audit events and state snapshots.
    persistence: Arc<Mutex<SqlitePersistence>>,
}

/// API request for registering a user.
///
/// This includes authentication information in addition to the user data.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RegisterUserApiRequest {
    /// The actor ID performing this action.
    actor_id: String,
    /// The role of the actor.
    actor_role: String,
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
    /// The actor ID performing this action.
    actor_id: String,
    /// The role of the actor.
    actor_role: String,
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
    /// The actor ID performing this action.
    actor_id: String,
    /// The role of the actor.
    actor_role: String,
    /// The cause ID for this action.
    cause_id: String,
    /// The cause description.
    cause_description: String,
    /// The year value (e.g., 2026).
    year: u16,
}

/// API request for creating an area.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct CreateAreaApiRequest {
    /// The actor ID performing this action.
    actor_id: String,
    /// The role of the actor.
    actor_role: String,
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

/// API response for listing bid years.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListBidYearsApiResponse {
    /// The list of bid years.
    bid_years: Vec<u16>,
}

/// API response for listing areas.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListAreasApiResponse {
    /// The bid year.
    bid_year: u16,
    /// The list of area identifiers.
    areas: Vec<String>,
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

/// Parses a role string into a Role enum.
fn parse_role(role_str: &str) -> Result<Role, HttpError> {
    match role_str.to_lowercase().as_str() {
        "admin" => Ok(Role::Admin),
        "bidder" => Ok(Role::Bidder),
        _ => Err(HttpError {
            status: StatusCode::BAD_REQUEST,
            message: format!("Invalid role: '{role_str}'. Must be 'admin' or 'bidder'"),
        }),
    }
}

/// Handler for POST `/bid_years` endpoint.
///
/// Creates a new bid year.
async fn handle_create_bid_year(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<CreateBidYearApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_id = %req.actor_id,
        role = %req.actor_role,
        year = req.year,
        "Handling create_bid_year request"
    );

    // Parse and authenticate
    let role: Role = parse_role(&req.actor_role)?;
    let actor: AuthenticatedActor =
        authenticate_stub(req.actor_id.clone(), role).map_err(|e| HttpError {
            status: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
        })?;

    let cause: Cause = Cause::new(req.cause_id, req.cause_description);

    // Get current bootstrap metadata
    let persistence = app_state.persistence.lock().await;
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    drop(persistence);

    // Build API request
    let create_request: CreateBidYearRequest = CreateBidYearRequest { year: req.year };

    // Execute command via API
    let bootstrap_result: BootstrapResult =
        create_bid_year(&metadata, &create_request, &actor, cause)?;

    // Persist the bootstrap result
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_bootstrap(&bootstrap_result)?;
    drop(persistence);

    info!(
        event_id = event_id,
        year = req.year,
        "Successfully created bid year"
    );

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
    Json(req): Json<CreateAreaApiRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_id = %req.actor_id,
        role = %req.actor_role,
        bid_year = req.bid_year,
        area_id = %req.area_id,
        "Handling create_area request"
    );

    // Parse and authenticate
    let role: Role = parse_role(&req.actor_role)?;
    let actor: AuthenticatedActor =
        authenticate_stub(req.actor_id.clone(), role).map_err(|e| HttpError {
            status: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
        })?;

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
    let bootstrap_result: BootstrapResult = create_area(&metadata, create_request, &actor, cause)?;

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
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    drop(persistence);

    let response: ListBidYearsResponse = list_bid_years(&metadata);

    Ok(Json(ListBidYearsApiResponse {
        bid_years: response.bid_years,
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
    drop(persistence);

    let request: ListAreasRequest = ListAreasRequest {
        bid_year: query.bid_year,
    };
    let response: ListAreasResponse = list_areas(&metadata, &request);

    Ok(Json(ListAreasApiResponse {
        bid_year: response.bid_year,
        areas: response.areas,
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

    // Validate that the bid year and area exist
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata()?;
    if !metadata.has_bid_year(&bid_year) {
        return Err(HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("Bid year {} does not exist", query.bid_year),
        });
    }
    if !metadata.has_area(&bid_year, &area) {
        return Err(HttpError {
            status: StatusCode::NOT_FOUND,
            message: format!(
                "Area '{}' does not exist in bid year {}",
                query.area, query.bid_year
            ),
        });
    }

    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    let response: ListUsersResponse = list_users(&state);

    let users: Vec<UserInfoResponse> = response
        .users
        .into_iter()
        .map(|u| UserInfoResponse {
            initials: u.initials,
            name: u.name,
            crew: u.crew,
        })
        .collect();

    Ok(Json(ListUsersApiResponse {
        bid_year: response.bid_year,
        area: response.area,
        users,
    }))
}

/// Handler for POST `/register_user` endpoint.
///
/// Authenticates the actor, authorizes the action, and registers a new user.
async fn handle_register_user(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<RegisterUserApiRequest>,
) -> Result<Json<RegisterUserApiResponse>, HttpError> {
    info!(
        actor_id = %req.actor_id,
        role = %req.actor_role,
        initials = %req.initials,
        "Handling register_user request"
    );

    // Parse and authenticate
    let role: Role = parse_role(&req.actor_role)?;
    let actor: AuthenticatedActor =
        authenticate_stub(req.actor_id.clone(), role).map_err(|e| HttpError {
            status: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
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

    // Build API request
    let register_request: RegisterUserRequest = RegisterUserRequest {
        bid_year: req.bid_year,
        initials: req.initials,
        name: req.name,
        area: req.area,
        user_type: req.user_type,
        crew: req.crew,
        cumulative_natca_bu_date: req.cumulative_natca_bu_date,
        natca_bu_date: req.natca_bu_date,
        eod_faa_date: req.eod_faa_date,
        service_computation_date: req.service_computation_date,
        lottery_value: req.lottery_value,
    };

    // Execute command via API
    let result: ApiResult<RegisterUserResponse> =
        register_user(&metadata, &state, register_request, &actor, cause)?;

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
    Json(req): Json<AdminActionRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_id = %req.actor_id,
        role = %req.actor_role,
        bid_year = req.bid_year,
        area = %req.area,
        "Handling checkpoint request"
    );

    // Parse and authenticate
    let role: Role = parse_role(&req.actor_role)?;
    let actor: AuthenticatedActor =
        authenticate_stub(req.actor_id.clone(), role).map_err(|e| HttpError {
            status: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
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
    let result: TransitionResult = checkpoint(&metadata, &state, &actor, cause)?;

    // Persist the transition
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_transition(
        &result,
        SqlitePersistence::should_snapshot(&result.audit_event.action.name),
    )?;
    drop(persistence);

    info!(event_id = event_id, "Successfully created checkpoint");

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
    Json(req): Json<AdminActionRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_id = %req.actor_id,
        role = %req.actor_role,
        bid_year = req.bid_year,
        area = %req.area,
        "Handling finalize request"
    );

    // Parse and authenticate
    let role: Role = parse_role(&req.actor_role)?;
    let actor: AuthenticatedActor =
        authenticate_stub(req.actor_id.clone(), role).map_err(|e| HttpError {
            status: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
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
    let result: TransitionResult = finalize(&metadata, &state, &actor, cause)?;

    // Persist the transition
    let mut persistence = app_state.persistence.lock().await;
    let event_id: i64 = persistence.persist_transition(
        &result,
        SqlitePersistence::should_snapshot(&result.audit_event.action.name),
    )?;
    drop(persistence);

    info!(event_id = event_id, "Successfully finalized round");

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
    Json(req): Json<AdminActionRequest>,
) -> Result<Json<WriteResponse>, HttpError> {
    info!(
        actor_id = %req.actor_id,
        role = %req.actor_role,
        bid_year = req.bid_year,
        area = %req.area,
        target_event_id = ?req.target_event_id,
        "Handling rollback request"
    );

    let target_event_id: i64 = req.target_event_id.ok_or_else(|| HttpError {
        status: StatusCode::BAD_REQUEST,
        message: String::from("target_event_id is required for rollback"),
    })?;

    // Parse and authenticate
    let role: Role = parse_role(&req.actor_role)?;
    let actor: AuthenticatedActor =
        authenticate_stub(req.actor_id.clone(), role).map_err(|e| HttpError {
            status: StatusCode::UNAUTHORIZED,
            message: e.to_string(),
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
    let result: TransitionResult = rollback(&metadata, &state, target_event_id, &actor, cause)?;

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
    let state: State = persistence
        .get_current_state(&bid_year, &area)
        .unwrap_or_else(|_| State::new(bid_year.clone(), area.clone()));
    drop(persistence);

    let response: StateResponse = state_to_response(&state);

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
    let state: State = persistence.get_historical_state(&bid_year, &area, &params.timestamp)?;
    drop(persistence);

    let response: StateResponse = state_to_response(&state);

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

/// Builds the application router with all endpoints.
fn build_router(app_state: AppState) -> Router {
    Router::new()
        .route("/bid_years", post(handle_create_bid_year))
        .route("/bid_years", get(handle_list_bid_years))
        .route("/areas", post(handle_create_area))
        .route("/areas", get(handle_list_areas))
        .route("/users", get(handle_list_users))
        .route("/register_user", post(handle_register_user))
        .route("/checkpoint", post(handle_checkpoint))
        .route("/finalize", post(handle_finalize))
        .route("/rollback", post(handle_rollback))
        .route("/state/current", get(handle_get_current_state))
        .route("/state/historical", get(handle_get_historical_state))
        .route("/audit/timeline", get(handle_get_audit_timeline))
        .route("/audit/event/{event_id}", get(handle_get_audit_event))
        .with_state(app_state)
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
        }
    }

    /// Helper to create a test register user request.
    fn create_test_register_request(
        actor_id: &str,
        role: &str,
        initials: &str,
    ) -> RegisterUserApiRequest {
        RegisterUserApiRequest {
            actor_id: actor_id.to_string(),
            actor_role: role.to_string(),
            cause_id: String::from("test-cause"),
            cause_description: String::from("Test user registration"),
            bid_year: 2026,
            initials: initials.to_string(),
            name: String::from("Test User"),
            area: String::from("TestArea"),
            user_type: String::from("CPC"),
            crew: Some(1),
            cumulative_natca_bu_date: String::from("2020-01-01"),
            natca_bu_date: String::from("2020-01-01"),
            eod_faa_date: String::from("2020-01-01"),
            service_computation_date: String::from("2020-01-01"),
            lottery_value: None,
        }
    }

    #[tokio::test]
    async fn test_register_user_as_admin_succeeds() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state.clone());

        // Bootstrap: Create bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Bootstrap: Create area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("TestArea"),
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let req_body: RegisterUserApiRequest =
            create_test_register_request("admin1", "admin", "AB");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let api_response: RegisterUserApiResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert!(api_response.success);
        assert_eq!(api_response.initials, "AB");
        assert!(api_response.event_id > 0);
    }

    #[tokio::test]
    async fn test_register_user_as_bidder_fails() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let req_body: RegisterUserApiRequest =
            create_test_register_request("bidder1", "bidder", "XY");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error_response: ErrorResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert!(error_response.error);
        assert!(error_response.message.contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_unauthorized_action_does_not_mutate_state() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state.clone());

        // Try to register as bidder (should fail)
        let req_body: RegisterUserApiRequest =
            create_test_register_request("bidder1", "bidder", "XY");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);

        // Verify state was not mutated by checking the audit timeline is empty
        let timeline_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/audit/timeline?bid_year=2026&area=TestArea")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(timeline_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: Vec<AuditEventResponse> = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(events.len(), 0, "No audit events should have been created");
    }

    #[tokio::test]
    async fn test_successful_action_persists_to_audit_timeline() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // Bootstrap: Create bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Bootstrap: Create area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("TestArea"),
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Register a user as admin
        let req_body: RegisterUserApiRequest =
            create_test_register_request("admin1", "admin", "AB");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        // Verify the action appears in the audit timeline
        let timeline_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/audit/timeline?bid_year=2026&area=TestArea")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(timeline_response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(timeline_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: Vec<AuditEventResponse> = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            events.len(),
            2,
            "Two audit events should be in this area's timeline (CreateArea, RegisterUser)"
        );
        assert_eq!(events[1].action_name, "RegisterUser");
        assert_eq!(events[1].actor_id, "admin1");
    }

    #[tokio::test]
    async fn test_audit_event_contains_actor_attribution() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // Bootstrap: Create bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Bootstrap: Create area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("TestArea"),
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Register a user as admin
        let req_body: RegisterUserApiRequest =
            create_test_register_request("admin1", "admin", "AB");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        // Get the audit timeline and verify actor attribution
        let timeline_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/audit/timeline?bid_year=2026&area=TestArea")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body_bytes = axum::body::to_bytes(timeline_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: Vec<AuditEventResponse> = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[1].actor_id, "admin1");
        assert_eq!(events[1].actor_type, "admin");
    }

    #[tokio::test]
    async fn test_checkpoint_as_admin_succeeds() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let req_body: AdminActionRequest = AdminActionRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("test-checkpoint"),
            cause_description: String::from("Test checkpoint"),
            bid_year: 2026,
            area: String::from("TestArea"),
            target_event_id: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/checkpoint")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let write_response: WriteResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert!(write_response.success);
        assert!(write_response.event_id.is_some());
    }

    #[tokio::test]
    async fn test_checkpoint_as_bidder_fails() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let req_body: AdminActionRequest = AdminActionRequest {
            actor_id: String::from("bidder1"),
            actor_role: String::from("bidder"),
            cause_id: String::from("test-checkpoint"),
            cause_description: String::from("Test checkpoint"),
            bid_year: 2026,
            area: String::from("TestArea"),
            target_event_id: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/checkpoint")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_get_audit_event_by_id() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state.clone());

        // Bootstrap: Create bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Bootstrap: Create area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("TestArea"),
        };
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Register a user
        let req_body: RegisterUserApiRequest =
            create_test_register_request("admin1", "admin", "AB");

        let register_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(register_response.status(), HttpStatusCode::OK);
        let body_bytes = axum::body::to_bytes(register_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let register_result: RegisterUserApiResponse = serde_json::from_slice(&body_bytes).unwrap();
        let event_id: i64 = register_result.event_id;

        // Get the audit event by ID (which is the RegisterUser event, event_id 3)
        let event_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/audit/event/{event_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(event_response.status(), HttpStatusCode::OK);

        let event_bytes = axum::body::to_bytes(event_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let event: AuditEventResponse = serde_json::from_slice(&event_bytes).unwrap();

        assert_eq!(event.action_name, "RegisterUser");
        assert_eq!(event.actor_id, "admin1");
    }

    #[tokio::test]
    async fn test_invalid_role_returns_bad_request() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let req_body: RegisterUserApiRequest =
            create_test_register_request("user1", "invalid_role", "STU");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_bid_year_as_admin_succeeds() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let req_body: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create initial bid year"),
            year: 2026,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let write_response: WriteResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert!(write_response.success);
        assert!(write_response.event_id.is_some());
    }

    #[tokio::test]
    async fn test_create_bid_year_as_bidder_fails() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let req_body: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("bidder1"),
            actor_role: String::from("bidder"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create initial bid year"),
            year: 2026,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_list_bid_years_empty() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/bid_years")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_response: ListBidYearsApiResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(list_response.bid_years.len(), 0);
    }

    #[tokio::test]
    async fn test_list_bid_years_after_creation() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // 1. Create a bid year
        let create_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create initial bid year"),
            year: 2026,
        };

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), HttpStatusCode::OK);

        // 2. Create area
        // List bid years
        let list_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/bid_years")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_result: ListBidYearsApiResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(list_result.bid_years.len(), 1);
        assert_eq!(list_result.bid_years[0], 2026);
    }

    #[tokio::test]
    async fn test_create_area_as_admin_succeeds() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // First create a bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };

        let by_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(by_response.status(), HttpStatusCode::OK);

        // Create an area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("North"),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let write_response: WriteResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert!(write_response.success);
        assert!(write_response.event_id.is_some());
    }

    #[tokio::test]
    async fn test_create_area_without_bid_year_fails() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("North"),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_list_areas_empty() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/areas?bid_year=2026")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_response: ListAreasApiResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(list_response.areas.len(), 0);
    }

    #[tokio::test]
    async fn test_list_areas_after_creation() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // Create bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Create area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("North"),
        };

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List areas
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/areas?bid_year=2026")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_result: ListAreasApiResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(list_result.areas.len(), 1);
        assert_eq!(list_result.areas[0], "NORTH");
    }

    #[tokio::test]
    async fn test_list_users_empty() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // Create bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("test"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };

        let _by_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Create area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("test"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("North"),
        };

        let _area_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List users (should be empty)
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/users?bid_year=2026&area=North")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_response: ListUsersApiResponse = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(list_response.users.len(), 0);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn test_complete_bootstrap_workflow() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // 1. Create bid year
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };

        let by_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(by_response.status(), HttpStatusCode::OK);

        // 2. Create area
        let area_req: CreateAreaApiRequest = CreateAreaApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("bootstrap"),
            cause_description: String::from("Create area"),
            bid_year: 2026,
            area_id: String::from("TestArea"),
        };

        let area_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&area_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(area_response.status(), HttpStatusCode::OK);

        // 3. Register a user (using TestArea which now exists)
        let user_req: RegisterUserApiRequest =
            create_test_register_request("admin1", "admin", "AB");

        let user_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register_user")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&user_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(user_response.status(), HttpStatusCode::OK);

        let user_bytes = axum::body::to_bytes(user_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let user_result: RegisterUserApiResponse = serde_json::from_slice(&user_bytes).unwrap();
        assert!(user_result.success);

        // 4. Create a checkpoint to snapshot the state
        let checkpoint_req: AdminActionRequest = AdminActionRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("checkpoint"),
            cause_description: String::from("Snapshot after user registration"),
            bid_year: 2026,
            area: String::from("TestArea"),
            target_event_id: None,
        };
        let checkpoint_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/checkpoint")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&checkpoint_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(checkpoint_response.status(), HttpStatusCode::OK);

        // 7. List bid years
        let list_by = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/bid_years")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let by_bytes = axum::body::to_bytes(list_by.into_body(), usize::MAX)
            .await
            .unwrap();
        let by_list: ListBidYearsApiResponse = serde_json::from_slice(&by_bytes).unwrap();
        assert_eq!(by_list.bid_years.len(), 1);

        // 8. List areas
        let list_areas = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/areas?bid_year=2026")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let areas_bytes = axum::body::to_bytes(list_areas.into_body(), usize::MAX)
            .await
            .unwrap();
        let areas_list: ListAreasApiResponse = serde_json::from_slice(&areas_bytes).unwrap();
        assert_eq!(areas_list.areas.len(), 1);

        // 9. Verify audit timeline shows all bootstrap and user events
        let timeline_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/audit/timeline?bid_year=2026&area=TestArea")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let timeline_bytes = axum::body::to_bytes(timeline_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let timeline_events: Vec<AuditEventResponse> =
            serde_json::from_slice(&timeline_bytes).unwrap();
        // Should have: CreateArea, RegisterUser, Checkpoint
        assert_eq!(
            timeline_events.len(),
            3,
            "Expected CreateArea, RegisterUser, and Checkpoint events"
        );
    }

    #[tokio::test]
    async fn test_list_users_nonexistent_bid_year_returns_not_found() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state);

        // Try to list users for a bid year that doesn't exist
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/users?bid_year=9999&area=North")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_users_nonexistent_area_returns_not_found() {
        let app_state: AppState = create_test_app_state();
        let app: Router = build_router(app_state.clone());

        // Create bid year but no area
        let bid_year_req: CreateBidYearApiRequest = CreateBidYearApiRequest {
            actor_id: String::from("admin1"),
            actor_role: String::from("admin"),
            cause_id: String::from("test"),
            cause_description: String::from("Create bid year"),
            year: 2026,
        };

        let _by_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/bid_years")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&bid_year_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Try to list users for an area that doesn't exist
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/users?bid_year=2026&area=NonExistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::NOT_FOUND);
    }
}
