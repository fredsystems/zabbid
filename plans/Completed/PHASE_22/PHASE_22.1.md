# Phase 22.1 — Authentication Error Hardening

## Goal

Prevent leakage of security-sensitive information during authentication while preserving
explicit, structured errors everywhere else in the system.

This phase is strictly scoped to **authentication and session establishment**.
No domain validation behavior outside authentication is changed.

---

## Problem Statement

The current authentication flow exposes internal failure reasons to unauthenticated clients,
such as:

- Operator does not exist
- Incorrect password
- Operator is disabled

While helpful for debugging, this information constitutes **security-domain leakage** and
violates best practices at the authentication boundary.

---

## Scope

### In Scope

- Login (`POST /api/auth/login`)
- Session validation failures
- Authentication error mapping (API → UI)
- UI messaging for authentication failures
- Tests covering authentication error behavior

### Out of Scope

- Authorization errors (403 after authentication)
- Domain validation errors
- Bootstrap, bidding, or operator management logic
- Audit behavior
- Logging or internal diagnostics

---

## Behavioral Requirements

### Authentication Failures (Pre-Auth Boundary)

All authentication failures MUST:

- Return **HTTP 401 Unauthorized**
- Return a **generic error identifier**
- NOT distinguish between:
  - Unknown username
  - Incorrect password
  - Disabled operator
  - Expired or invalid credentials

#### Canonical API Response

```json
HTTP 401
{
  "error": "invalid_credentials"
}
```

UI Messaging

UI MUST display a generic message such as:

“Invalid username or password.”

No other details may be shown to unauthenticated users.

Authorization Failures (Post-Auth Boundary)

Authorization failures remain unchanged:

HTTP 403 Forbidden

Explicit, actionable messaging allowed

Role-based failures may be surfaced clearly

Internal Error Handling

Internal authentication error variants MAY remain distinct:

OperatorNotFound

InvalidPassword

OperatorDisabled

These errors MAY be:

Logged

Counted

Tested internally

They MUST NOT be exposed via API responses or UI messaging

API Layer Requirements

Introduce a single external error mapping for authentication failures

All internal auth errors map to invalid_credentials

No branching on error cause when constructing API responses

UI Requirements

Remove conditional messaging based on auth failure reason

Display a single generic error message for all login failures

Preserve existing visual styling and error presentation

Testing Requirements

Tests must verify:

All authentication failure cases return identical API responses

HTTP status is always 401

Error payload does not vary by failure cause

UI does not display differentiated error messages

Authorization (403) behavior remains unchanged

Exit Criteria

Phase 22.1 is complete when:

Authentication errors no longer leak internal failure causes

API responses for auth failures are uniform

UI displays a single generic auth failure message

Authorization errors remain explicit

No other domain or UI behavior changes

All tests pass

cargo xtask ci passes

pre-commit run --all-files passes
