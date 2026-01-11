// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Session extraction and authentication middleware for the server.
//!
//! This module provides Axum extractors for validating session tokens
//! and enforcing authentication at the server boundary.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use tracing::{debug, warn};
use zab_bid_api::{AuthenticatedActor, AuthenticationService};
use zab_bid_persistence::OperatorData;

use crate::AppState;

/// Extractor for authenticated operators.
///
/// This extractor validates the session token from the Authorization header
/// and returns the authenticated operator context.
///
/// # Usage
///
/// ```ignore
/// async fn my_handler(
///     SessionOperator(actor, operator): SessionOperator,
/// ) -> Result<Json<Response>, HttpError> {
///     // actor: AuthenticatedActor
///     // operator: OperatorData
///     Ok(Json(Response { ... }))
/// }
/// ```
///
/// # Authentication Flow
///
/// 1. Extract `Authorization: Bearer <token>` header
/// 2. Validate session token via `AuthenticationService::validate_session`
/// 3. Check session expiration
/// 4. Check operator disabled status
/// 5. Return `AuthenticatedActor` and `OperatorData`
///
/// # Errors
///
/// Returns HTTP 401 Unauthorized if:
/// - Authorization header is missing
/// - Authorization header format is invalid
/// - Session token is invalid
/// - Session is expired
/// - Operator is disabled
pub struct SessionOperator(pub AuthenticatedActor, pub OperatorData);

impl FromRequestParts<AppState> for SessionOperator {
    type Rejection = SessionError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .ok_or_else(|| {
                debug!("Missing Authorization header");
                SessionError::MissingAuthorizationHeader
            })?
            .to_str()
            .map_err(|_| {
                warn!("Invalid Authorization header encoding");
                SessionError::InvalidAuthorizationHeader
            })?;

        // Parse Bearer token
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            warn!("Authorization header does not start with 'Bearer '");
            SessionError::InvalidAuthorizationHeader
        })?;

        // Validate session
        let mut persistence = state.persistence.lock().await;
        let (actor, operator) = AuthenticationService::validate_session(&mut persistence, token)
            .map_err(|e| {
                warn!(error = %e, "Session validation failed");
                SessionError::InvalidSession(e.to_string())
            })?;

        debug!(
            login_name = %operator.login_name,
            role = ?actor.role,
            "Session validated successfully"
        );

        Ok(Self(actor, operator))
    }
}

/// Session extraction errors.
///
/// These errors are returned when session validation fails and are
/// automatically converted to HTTP responses.
#[derive(Debug)]
pub enum SessionError {
    /// Authorization header is missing.
    MissingAuthorizationHeader,
    /// Authorization header format is invalid.
    InvalidAuthorizationHeader,
    /// Session validation failed.
    InvalidSession(String),
}

impl IntoResponse for SessionError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingAuthorizationHeader => {
                (StatusCode::UNAUTHORIZED, "Missing Authorization header")
            }
            Self::InvalidAuthorizationHeader => (
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header format. Expected: 'Bearer <token>'",
            ),
            Self::InvalidSession(reason) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    format!("Session validation failed: {reason}"),
                )
                    .into_response();
            }
        };

        (status, message).into_response()
    }
}
