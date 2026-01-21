# Phase 29F — Bid Status Tracking Structure

## Purpose

Implement bid status tracking infrastructure to record and manage user bidding progress through rounds.

This sub-phase creates the **structure only**. Status transitions are operator-initiated; the system never advances status based on time alone.

---

## Scope

### 1. Bid Status Enumeration

Define the following statuses (tracked per user, per round, per area):

- **Not Started (Pre-Window)** — user's bid window has not yet begun
- **Not Started (In Window)** — user's bid window is active, but they haven't started bidding
- **In Progress** — user has started but not completed their bids
- **Completed (On Time)** — user completed bids before window closed
- **Completed (Late)** — user completed bids after window closed
- **Missed** — user did not bid (no call / management pause)
- **Voluntarily Not Bidding** — user explicitly opted out
- **Proxy** — bids entered by proxy (on behalf of user)

### 2. Database Schema

#### Bid Status Table

```sql
CREATE TABLE bid_status (
    bid_status_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    round_id INTEGER NOT NULL,
    status TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    updated_by INTEGER NOT NULL,
    notes TEXT,
    UNIQUE (bid_year_id, area_id, user_id, round_id),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id),
    FOREIGN KEY(user_id) REFERENCES users(user_id),
    FOREIGN KEY(round_id) REFERENCES rounds(round_id),
    FOREIGN KEY(updated_by) REFERENCES operators(operator_id)
);
```

#### Status Transition History Table

```sql
CREATE TABLE bid_status_history (
    history_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_status_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    previous_status TEXT,
    new_status TEXT NOT NULL,
    transitioned_at TEXT NOT NULL,
    transitioned_by INTEGER NOT NULL,
    notes TEXT,
    FOREIGN KEY(bid_status_id) REFERENCES bid_status(bid_status_id),
    FOREIGN KEY(audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY(transitioned_by) REFERENCES operators(operator_id)
);
```

### 3. Domain Types

Add to `domain/src/types.rs`:

```rust
pub enum BidStatus {
    NotStartedPreWindow,
    NotStartedInWindow,
    InProgress,
    CompletedOnTime,
    CompletedLate,
    Missed,
    VoluntarilyNotBidding,
    Proxy,
}

pub struct UserBidStatus {
    user_id: i64,
    round_id: i64,
    status: BidStatus,
    updated_at: DateTime<Utc>,
    updated_by: i64,
    notes: Option<String>,
}
```

### 4. Status Lifecycle Rules

#### Initial Status

- Status history begins **only after confirmation** (Sub-Phase 29E)
- Initial status for all users: `NotStartedPreWindow`
- Status is created automatically at confirmation for all user/round combinations

#### Valid Transitions

Define permitted status transitions:

- `NotStartedPreWindow → NotStartedInWindow` (automatic when window opens)
- `NotStartedInWindow → InProgress` (operator marks)
- `NotStartedInWindow → VoluntarilyNotBidding` (operator marks)
- `NotStartedInWindow → Proxy` (operator marks)
- `InProgress → CompletedOnTime` (operator marks, within window)
- `InProgress → CompletedLate` (operator marks, after window)
- `NotStartedInWindow → Missed` (operator marks)
- `NotStartedInWindow → CompletedOnTime` (direct completion)
- `NotStartedInWindow → CompletedLate` (direct late completion)

**Invalid transitions:**

- Any transition from terminal states (`CompletedOnTime`, `CompletedLate`, `Missed`, `VoluntarilyNotBidding`, `Proxy`)
- Transition from `InProgress` to `NotStarted*`
- Transition from `Missed` to any other state

### 5. Time-Based Status Updates

**Critical Rule:** The system **never** automatically advances status based on time.

- `NotStartedPreWindow → NotStartedInWindow` may be computed/displayed based on current time, but is **not persisted automatically**
- Operators must explicitly trigger status updates
- "Window open" or "window closed" are informational only

### 6. API Endpoints

#### Get Bid Status

- `GET /api/bid-years/{bid_year_id}/areas/{area_id}/bid-status`
  - Returns bid status for all users in area, all rounds
  - Includes computed time-based context (e.g., "window is open")

- `GET /api/bid-years/{bid_year_id}/rounds/{round_id}/users/{user_id}/bid-status`
  - Returns bid status for specific user in specific round

#### Update Bid Status

- `POST /api/bid-status/{bid_status_id}/transition`
  - Request: `{ new_status: string, notes: string }`
  - Validates transition is permitted
  - Records transition in history
  - Records audit event
  - Requires Bidder or Admin role

#### Bulk Status Update

- `POST /api/bid-years/{bid_year_id}/rounds/{round_id}/bulk-status-update`
  - Request: `{ user_ids: [i64], new_status: string, notes: string }`
  - Updates status for multiple users at once
  - Validates all transitions are permitted
  - Atomic operation (all or nothing)

### 7. Audit Events

All status transitions generate audit events:

- `action = "BidStatusTransition"`
- `actor = <operator>`
- `cause = "Status update by operator"`
- `state_before = { user_id, round_id, status: "NotStartedInWindow" }`
- `state_after = { user_id, round_id, status: "InProgress" }`

### 8. Lifecycle Constraints

- Bid status tracking is **only active** after confirmation (Canonicalized state)
- Status cannot be created or modified before confirmation
- Status history is immutable once created (transitions are additive)

### 9. Persistence Layer

- Add insert support for initial status creation (at confirmation)
- Add update support for status transitions
- Add read support for status queries
- Add history tracking for all transitions
- Enforce valid transition constraints at persistence boundary

---

## Explicit Non-Goals

- No automatic time-based status transitions
- No bid execution logic
- No notification system
- No UI for status dashboard (out of scope for Phase 29)
- No status rollback or correction mechanism
- No status-based workflow automation

---

## Completion Checklist

- [ ] Migrations created for both SQLite and MySQL (bid_status, bid_status_history)
- [ ] Schema verification passes (`cargo xtask verify-migrations`)
- [ ] Domain types created (BidStatus enum, UserBidStatus)
- [ ] Status lifecycle rules defined and documented
- [ ] Valid transition matrix implemented
- [ ] Persistence layer supports status CRUD
- [ ] API endpoints implemented (get, update, bulk)
- [ ] Audit event recording implemented
- [ ] Initial status creation at confirmation (integration with Sub-Phase 29E)
- [ ] Unit tests for status transitions
- [ ] Tests for invalid transitions (must fail)
- [ ] Integration tests for API endpoints
- [ ] Tests for bulk updates
- [ ] Tests for audit event generation
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Status lifecycle rules conflict with existing domain rules
- Transition validation logic is unclear or ambiguous
- Integration with confirmation (Sub-Phase 29E) is uncertain
- Audit event structure for status transitions is unclear
- Bulk update atomicity requirements conflict with persistence patterns
- Terminal state semantics require clarification

---

## Risk Notes

- Initial status creation at confirmation may be expensive for large datasets
- Status history table may grow large over time
- Bulk updates may need transaction management
- Invalid transition enforcement must be airtight (no backdoors)
- Existing bid years will have no status tracking until confirmed
- Status queries may be slow without proper indexing
