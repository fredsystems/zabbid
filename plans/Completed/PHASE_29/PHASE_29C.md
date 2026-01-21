# Phase 29C — Bid Schedule Declaration

## Purpose

Implement bid schedule declaration infrastructure required for determining bid windows.

The bid schedule defines:

- **When** bidding begins (start date, timezone)
- **Daily bid window** (wall-clock times)
- **Capacity** (bidders per area per day)

This sub-phase creates the **structure and validation only**. Window calculation is deferred to Sub-Phase 29E.

---

## Scope

### 1. Database Schema

Add to `bid_years` table:

```sql
ALTER TABLE bid_years ADD COLUMN bid_timezone TEXT;
ALTER TABLE bid_years ADD COLUMN bid_start_date TEXT;
ALTER TABLE bid_years ADD COLUMN bid_window_start_time TEXT;
ALTER TABLE bid_years ADD COLUMN bid_window_end_time TEXT;
ALTER TABLE bid_years ADD COLUMN bidders_per_area_per_day INTEGER;
```

**All fields are nullable until confirmation.**

At confirmation time, all fields must be non-null.

### 2. Domain Types

Add to `domain/src/types.rs`:

```rust
pub struct BidSchedule {
    /// IANA timezone identifier (e.g., "America/New_York")
    timezone: String,
    /// Bid start date (must be a Monday, must be in the future at confirmation)
    start_date: Date,
    /// Daily bid window start time (wall-clock)
    window_start_time: Time,
    /// Daily bid window end time (wall-clock)
    window_end_time: Time,
    /// Number of bidders per area per day
    bidders_per_day: u32,
}
```

### 3. Validation Rules

#### Timezone

- Must be a valid IANA timezone identifier
- No implicit defaults
- Validate using `chrono-tz` or equivalent

#### Start Date

- Must be a Monday
- Must be in the future (at confirmation time)
- Date-only (no time component)

#### Daily Bid Window

- `window_start_time` must be before `window_end_time`
- Times are wall-clock (e.g., "08:00:00", "18:00:00")
- Uniform across all areas
- No timezone offset stored (interpreted in declared timezone)

#### Bidders Per Day

- Must be > 0
- Used to derive individual bid windows later

### 4. Time Semantics (Normative)

**All bid times are wall-clock times.**

- Nominal labels define windows
- DST transitions:
  - do **not** shift labels
  - may change duration
  - must **never** make users early or late

**Example:** If bid window is "08:00–18:00" and DST ends at 2am, the window is still "08:00–18:00" (but is 1 hour longer in duration).

Execution logic (not this sub-phase) **must** use timezone-aware arithmetic.

### 5. API Endpoints

#### Set Bid Schedule

- `POST /api/bid-years/{bid_year_id}/bid-schedule`
  - Request: `{ timezone, start_date, window_start_time, window_end_time, bidders_per_day }`
  - Validates all fields
  - Pre-confirmation: editable
  - Post-confirmation: immutable

#### Get Bid Schedule

- `GET /api/bid-years/{bid_year_id}/bid-schedule`
  - Returns current bid schedule (or null if not set)

### 6. API Response Updates

Update `BidYearInfo` to include:

```rust
pub bid_schedule: Option<BidScheduleInfo>

pub struct BidScheduleInfo {
    pub timezone: String,
    pub start_date: String,
    pub window_start_time: String,
    pub window_end_time: String,
    pub bidders_per_day: u32,
}
```

### 7. Lifecycle Constraints

- Bid schedule is editable in `Draft` and `BootstrapComplete` states
- After confirmation/canonicalization, bid schedule becomes immutable
- Readiness evaluation (Sub-Phase 29D) must check that bid schedule is set and valid

### 8. Persistence Layer

- Add insert/update support for new fields
- Add read support for new fields
- Store times as TEXT in ISO 8601 format (HH:MM:SS)
- Store timezone as TEXT (IANA identifier)

---

## Explicit Non-Goals

- No bid window calculation (that's Sub-Phase 29E)
- No bid execution logic
- No status tracking
- No user assignment to windows
- No timezone conversion logic (validation only)
- No UI for bid schedule management (out of scope for Phase 29)

---

## Completion Checklist

- [ ] Migrations created for both SQLite and MySQL
- [ ] Schema verification passes (`cargo xtask verify-migrations`)
- [ ] Domain types created
- [ ] Timezone validation implemented
- [ ] Start date validation (Monday, future)
- [ ] Daily window validation (start < end)
- [ ] Persistence layer supports new fields
- [ ] API endpoints implemented
- [ ] API response types updated
- [ ] Lifecycle constraints enforced
- [ ] Unit tests for validation rules
- [ ] Integration tests for API endpoints
- [ ] Tests for invalid timezones, dates, times
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Timezone validation library is unavailable or unclear
- Time representation conflicts with existing patterns
- Start date "future" validation semantics are ambiguous (relative to what timestamp?)
- Post-confirmation mutability requirements are uncertain
- DST semantics require additional clarification

---

## Risk Notes

- Existing bid years will have null bid schedule fields until explicitly set
- Readiness evaluation (later sub-phase) will fail until bid schedule is configured
- Time parsing and validation may require new dependencies (chrono-tz)
- "Future" validation at confirmation time may be tricky (server time vs. declared timezone)
