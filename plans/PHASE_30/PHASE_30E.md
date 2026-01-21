# Phase 30E — Bid Schedule UI

## Purpose

Implement the UI for declaring and managing the bid schedule, including
timezone, start date, daily bid window, and bidders per area per day.

This sub-phase delivers the **bid timing configuration workflow** required
to complete pre-bid setup and enable bid window derivation.

---

## Scope

### A. Bid Schedule Component

Create `ui/src/components/BidScheduleSetup.tsx`
(if not already created during Phase 30D).

This component must:

1. **Display current bid schedule** (if set)
   - Bid timezone (IANA identifier)
   - Bid start date (YYYY-MM-DD)
   - Daily bid window (wall-clock start and end times)
   - Bidders per area per day
   - Audit metadata (last updated time and actor, if available)

2. **Set or edit bid schedule**
   - Single form covering all required fields
   - Validation rules enforced at input time (see below)
   - Lifecycle constraint: editable until bidding commences
   - Clear submission, success, and error handling

3. **Schedule validation (UI-level)**
   - Timezone must be a valid IANA identifier
   - Start date must be a Monday
   - Start date **may be in the past at edit time**, but must be
     valid at confirmation time
   - Daily window start must be before end
   - Bidders per area per day must be a positive integer
   - All fields required before readiness can be achieved

4. **Wall-clock semantics explanation**
   - Explicit explanation that times are wall-clock in selected timezone
   - Clear statement that DST does not shift clock labels
   - No elapsed-duration calculations displayed or implied

5. **Lifecycle awareness**
   - Clearly show when schedule is locked
   - Disable inputs when locked
   - Preserve read-only display after lock

---

### B. Form Fields

#### Timezone Selection

- UI element: searchable dropdown or autocomplete
- Data source: IANA timezone list
- Common timezones surfaced (e.g., U.S. timezones)
- Display format: `America/New_York (Eastern Time)`
- Validation: must be valid IANA identifier

Implementation note: use a curated list or library to avoid overwhelming users.

---

#### Start Date Selection

- UI element: date picker
- Constraints:
  - Must be a Monday (hard validation error)
  - Display weekday prominently
  - Past dates allowed at edit time but flagged visually
- Format: ISO 8601 date-only (`YYYY-MM-DD`)

**Note:** “Future date” is enforced at _confirmation_, not at edit time.

---

#### Daily Bid Window

- Start time: time picker
- End time: time picker
- Validation:
  - End time must be after start time
- Display: wall-clock times in selected timezone
- Help text: “Applies uniformly to all bid days”

Do **not** support day-spanning windows.
End-before-start is a validation error.

---

#### Bidders Per Area Per Day

- UI element: number input
- Validation: integer ≥ 1
- Help text: “Maximum users who may bid per area per calendar day”
- Optional suggested default: 10 (non-binding)

---

### C. Backend API Integration

Frontend bindings assumed to exist or be added:

```ts
export async function setBidSchedule(
  sessionToken: string,
  bidYearId: number,
  timezone: string,
  startDate: string,
  dailyStartTime: string,
  dailyEndTime: string,
  biddersPerAreaPerDay: number,
): Promise<SetBidScheduleResponse>;

export async function getBidSchedule(
  sessionToken: string,
  bidYearId: number,
): Promise<GetBidScheduleResponse>;
```

Corresponding types in `ui/src/types.ts`:

- `BidScheduleInfo`
- `SetBidScheduleRequest`
- `SetBidScheduleResponse`
- `GetBidScheduleResponse`

---

### D. Bootstrap Workflow Integration

If Phase 30D is implemented:

- Route: `/admin/bootstrap/schedule`
- Positioned after area → round group assignment
- Before readiness review

Otherwise:

- Add standalone route
- Ensure readiness links route here when schedule missing

---

### E. Time Semantics (Normative)

All bid schedule times are **wall-clock times** in the selected timezone.

UI must **not**:

- Convert times to UTC for display
- Imply elapsed duration semantics
- Hide or downplay timezone context

UI must:

- Display timezone prominently
- Show times exactly as entered
- Explain DST behavior explicitly

**Example display:**

```text
Bid Schedule
Timezone: America/New_York (Eastern Time)
Start Date: 2026-03-09 (Monday)
Daily Window: 08:00 – 17:00 (local wall-clock)
Bidders Per Area Per Day: 10

Note: During DST transitions, the bidding window
remains 08:00–17:00 local time each day.
```

---

### F. Validation Error Messaging

- Invalid timezone: “Please select a valid timezone”
- Start date not Monday: “Start date must be a Monday”
- Start date in past: visual warning only (hard error at confirmation)
- End before start: “End time must be after start time”
- Invalid bidders count: “Must be a positive integer”

---

### G. Styling

Use SCSS modules:

- `ui/src/styles/bid-schedule-setup.module.scss`

Follow AGENTS.md:

- Mobile-first layout
- Full-width fields on mobile
- Touch-friendly date/time pickers
- No inline styles
- Clear help text and warnings

---

## UI Design Constraints

### Mobile-First

- Single-column layout
- Native pickers preferred
- Timezone selector searchable
- Long help text collapsible

### Lifecycle Awareness

- Lifecycle badge visible
- Inputs disabled when locked
- Read-only display preserved post-lock

### Readiness Integration

- Missing schedule is a readiness blocker
- Completion reflected immediately
- Readiness links route here when blocked

---

## Validation & Testing

### Manual Validation

1. Create schedule
2. Edit schedule
3. Test invalid weekdays
4. Test window validation
5. Test bidders per day
6. Verify lifecycle lock
7. Verify DST explanatory text
8. Test mobile layout
9. Test multiple timezones

---

## Backend Dependency Verification

Before implementation, verify:

1. Schedule set/get endpoints exist
2. Backend validates Monday rule
3. Backend enforces lifecycle locks
4. Backend stores timezone as IANA ID
5. Backend stores times as wall-clock values

If not, **stop and document the gap**.

---

## Explicit Non-Goals

- No automatic DST adjustments
- No duration math
- No bid window preview logic
- No multi-timezone schedules
- No backend changes
- No domain logic changes

---

## Completion Conditions

This sub-phase is complete when:

- Schedule UI exists and is usable
- All validation rules enforced correctly
- Wall-clock semantics clear
- Lifecycle locking respected
- Backend integration complete
- Mobile usability verified
- Manual validation passes
- CI and pre-commit pass
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Backend schedule API missing or inconsistent
- DST semantics unclear or violated
- Lifecycle enforcement missing
- UI implies elapsed-time semantics

---

## Risk Notes

- Time handling is subtle and user-visible
- Wall-clock semantics must remain explicit
- Confirmation-time validation may require coordination with readiness logic
