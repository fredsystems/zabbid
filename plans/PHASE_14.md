# Phase

## Phase 14: Operator Identity, Authentication, and Authorization

### Phase 14 Goal

Introduce **first-class operator identity** (admins and bidders), **session-based authentication**, and **role-based authorization** for all state-changing actions, while preserving:

- audit immutability
- canonical-vs-derived separation
- deterministic domain behavior
- “no deletion after audit involvement” invariants

This phase exists to make the system safe to operate by multiple trusted humans without losing forensic clarity.

---

### Phase 14 Scope

Phase 14 includes:

- Canonical persistence for **operators**
- Session-based authentication suitable for browser UI usage
- Role-based authorization for write endpoints:
  - **Admin**: all structural and corrective actions
  - **Bidder**: limited allowed actions (initially crew changes + bid operations later)
- Audit events that always attribute:
  - `actor_operator_id` (and stable actor display fields)
  - `target_*_id` when an action mutates a target entity (e.g., user changes)
- Operator disablement (soft removal) without breaking audit referential integrity
- Session expiration and renewal rules (simple + deterministic)
- UI integration to:
  - log in / log out
  - show current operator identity + role
  - include actor/cause defaults for commands

Phase 14 explicitly excludes:

- 2FA / WebAuthn / SSO
- fine-grained permissions beyond Admin/Bidder
- external identity providers
- bid entry UX (unless needed to validate bidder permissions)
- multi-bid-year workspaces (system remains one active bid year at a time)
- performance optimization

---

### Phase 14 Domain Definitions

#### Operator

An **Operator** is a trusted system user who performs actions in the system.
Operators are distinct from domain users whose bids are entered.

Operator fields include:

- stable internal identifier: `operator_id` (opaque UUID/ULID/integer)
- `login_name` (unique, case-insensitive)
- `display_name` (mutable)
- `role`: `Admin | Bidder`
- `is_disabled`: bool
- timestamps as needed (created_at, disabled_at, last_login_at)

Operators are:

- **never deleted** after they are referenced by any audit event
- **deletable** only if they have never appeared in any audit event

#### Sessions

A **Session** represents authenticated operator access:

- created at login
- tied to exactly one operator
- expires after an inactivity window (e.g., 30 days) and/or absolute lifetime (optional)
- stored server-side
- represented to the browser as an opaque session token (cookie or Authorization header)

Sessions:

- may be deleted at any time
- are not audited
- do not mutate domain state

---

### Phase 14 Persistence & Structural Guarantees

This phase must enforce the following using **database constraints**, not just code:

- `operators.login_name` is unique (case-insensitive normalization required)
- audit events reference operators via a foreign key:
  - `audit_events.actor_operator_id -> operators.operator_id`
  - deletion is restricted when referenced (`ON DELETE RESTRICT`)
- operators may be disabled but remain referenceable forever
- sessions reference operators and are deleted when expired/invalidated

SQLite requirements:

- foreign key enforcement must be enabled and treated as a startup invariant:
  - if FK enforcement is not active, the server must refuse to start

---

### Phase 14 Authorization Rules

Authorization is enforced **before** command execution:

#### Admin may

- create/modify/deprecate/seal bid years
- create/modify/disable areas (with constraints)
- create/modify/move users (with constraints)
- operator management actions
- rollback/checkpoint/finalize

#### Bidder may

- edit user crew assignment
- (future) enter/modify/withdraw bids
- (future) actions explicitly delegated to bidder role

Domain/core logic remains role-agnostic.
Authorization exists only at the boundary (API/server).

---

### Phase 14 API Surface

Phase 14 introduces:

- `POST /auth/login`
- `POST /auth/logout`
- `GET /auth/me` (who am I, role, is_disabled, session expiry info)

And updates write endpoints to:

- require an authenticated session
- derive `actor` from session, not from request body
- still require a **cause** (provided by UI/operator, with defaults allowed)

Actor Envelope changes:

- remove `actor_id` and `actor_role` from operator-provided payloads
- keep `cause_id` and `cause_description` required (or explicitly defaultable)

---

### Phase 14 Audit Requirements

All successful state mutations must emit exactly one audit event including:

- `actor_operator_id`
- stable actor snapshot fields (at least `login_name`, optionally `display_name`)
- `cause_id`, `cause_description`
- action type + structured action payload
- when applicable: `target_user_id`, `target_area_id`, `target_bid_year_id`

Failed actions emit no audit events.

---

### Phase 14 UI Requirements

UI must:

- require login before showing operator workflows
- display current operator identity and role
- provide editable cause defaults (visible, not hidden)
- show explicit authorization failures (“You are not permitted to do that”)
- handle disabled operator sessions gracefully (force logout / show blocked state)

---

### Phase 14 Testing Requirements

Tests must demonstrate:

- operator creation with normalization rules (case-insensitive uniqueness)
- login produces a valid session token
- expired/invalid sessions are rejected deterministically
- disabled operators cannot authenticate or cannot execute writes (choose one and enforce)
- Admin vs Bidder authorization enforced at API boundary
- audit events reference operators correctly and prevent deletion once referenced
- write endpoints reject unauthenticated requests
- FK enforcement startup check is covered (unit or integration)

All tests must be deterministic and run via `cargo xtask ci`.

---

### Phase 14 Exit Criteria

Phase 14 is complete when all of the following are true:

- operators exist as first-class canonical entities
- authentication works in browser UI via sessions
- Admin/Bidder authorization gates all write endpoints
- actor identity is derived from session (not request payload)
- audit events include actor operator id + target ids when applicable
- operators cannot be deleted after being referenced in audit events
- disabled operators are supported without loss of audit integrity
- UI login + operator header + auth failures are functional
- `api_cli.py` is updated to match all API changes
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
