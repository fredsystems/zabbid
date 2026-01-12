# Phase 15

## Phase 15 — Operator Authentication Bootstrap & Gated Admin UI

### Phase Goal

Establish a **secure, explicit bootstrap path** for operator authentication and gate _all existing admin UI functionality_ behind authenticated operator access, while keeping the public UI surface completely unauthenticated.

This phase formalizes:

- Operator login UX
- Initial system bootstrap behavior
- Separation of public vs admin UI
- One-time bootstrap credentials with forced replacement

---

## Phase 15 Scope

### Included

- Public vs admin UI separation
- Operator authentication UI at `/admin`
- One-time bootstrap login (`admin` / `admin`)
- Forced creation of first real admin operator
- Session-based authentication for all admin UI
- Proper handling of backend connectivity failures
- Explicit operator identity display in UI

### Explicitly Excluded

- Public (non-admin) functionality beyond a placeholder landing page
- Bid editing or bidding workflows
- Permission granularity beyond Admin vs Bidder
- External identity providers
- Password recovery flows

---

## UI Surface Definitions

### `/` — Public UI (Unauthenticated)

- Displays static placeholder content:
  - “Welcome to ZAB Bidding”
- No login buttons
- No links to admin functionality
- No authentication logic

This surface is intentionally minimal and future-facing.

---

### `/admin` — Operator UI (Authenticated)

- Entire admin UI is mounted under `/admin`
- No admin UI content renders unless authenticated
- All existing operator workflows live exclusively here

---

## Authentication Model

### Operators

- Operators are **canonical entities**
- Operators are uniquely identifiable
- Operators have:
  - `login_name`
  - `display_name`
  - `role` (Admin | Bidder)
  - `is_disabled`
- Operators may be deleted **only if they have never appeared in an audit event**
- Operators referenced by audit events are protected by FK constraints

---

### Sessions

- Sessions are:
  - Created on login
  - Stored server-side
  - Time-limited
  - Revocable on logout
- Session expiry is enforced automatically
- Expired sessions require reauthentication

---

## Bootstrap Authentication (Critical)

### Bootstrap Condition

Bootstrap mode is active **only if**:

- The `operators` table is empty

---

### Bootstrap Login

When in bootstrap mode:

- The system accepts exactly:
  - Username: `admin`
  - Password: `admin`
- This login:
  - Does **not** represent a real operator
  - Does **not** persist as an operator record
  - Exists only to unlock bootstrap flow

---

### Bootstrap Flow (Required)

After successful `admin/admin` authentication:

1. UI immediately presents **Create Initial Admin** screen
2. Required fields:
   - New admin username
   - New admin display name
   - New password
   - Password confirmation
3. Validation:
   - Passwords must match
   - Username must be valid and unique
4. On successful submission:
   - A **real Admin operator** is created
   - The bootstrap session is terminated
   - The user is logged out automatically
   - UI redirects to standard `/admin` login screen
5. From this point forward:
   - `admin/admin` is permanently disabled
   - All logins use real operator credentials only

---

### Prohibited Actions During Bootstrap

While in bootstrap mode:

- No bid year creation
- No area creation
- No user creation
- No state-changing actions of any kind

Only operator bootstrap is permitted.

---

## Admin Login (Normal Operation)

When at least one operator exists:

- `/admin` presents standard login screen
- Requires valid operator credentials
- Disabled operators are rejected with clear feedback
- Successful login establishes a session

---

## Authorization Rules

- All admin UI actions require a valid session
- Admin-only actions are enforced server-side
- Bidder sessions are rejected from Admin-only actions
- Authorization failures:
  - Return 403 Forbidden
  - Produce **no audit events**

---

## Operator Identity in UI

- The active operator identity is always visible:
  - login name
  - display name
  - role
- Logout is always available
- Disabled operator state is clearly indicated if applicable

---

## Error Handling & Connectivity

- Backend unreachable:
  - UI displays “Backend unavailable”
  - Automatic reconnect attempts occur at a reasonable interval
- On reconnection:
  - UI refreshes canonical state
  - Session validity is rechecked
- Authentication failures are explicit and actionable

---

## Audit Semantics

- All state-changing actions:
  - Attribute both:
    - acting operator (actor)
    - target entity (e.g. target user)
- Authentication failures:
  - Produce no audit events
- Bootstrap operator creation:
  - Is auditable as a system initialization event (implementation-defined)

---

## Phase 15 Exit Criteria

Phase 15 is complete when all of the following are true:

- `/` serves unauthenticated placeholder UI
- `/admin` is fully gated behind authentication
- Bootstrap login works **only** when no operators exist
- Bootstrap login forces creation of a real admin operator
- Bootstrap credentials cannot be reused
- Admin UI is inaccessible without authentication
- Operator identity is visible in UI
- Sessions expire and require reauthentication
- Disabled operators cannot log in
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
