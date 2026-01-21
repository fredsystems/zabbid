# Phase 30E — Bid Schedule UI

## Purpose

Implement the UI for declaring and managing the bid schedule, including
timezone, start date, daily bid window, and bidders per area per day.

This sub-phase delivers the **bid timing configuration workflow** required
to complete pre-bid setup and enable bid window calculation.

---

## Scope

### A. Bid Schedule Component

Create `ui/src/components/BidScheduleSetup.tsx` (if not already created in 30D).

This component must:

1. **Display current bid schedule** (if set)
   - Bid timezone (IANA identifier)
   - Bid start date (displayed as YYYY-MM-DD)
   - Daily bid window (wall-clock start and end times)
   - Bidders per area per day
   - Display audit metadata (when set, by whom)

2. **Set/Edit bid schedule**
   - Form to input all required fields
   - Validation rules (see below)
   - Lifecycle constraint: editable until bidding commences
   - Clear submission and error handling

3. **Schedule validation**
   - Timezone: must be valid IANA timezone
   - Start date: must be a Monday
   - Start date: must be in the future at confirmation time (warning if not)
   - Daily window: start time must be before end time
   - Bidders per day: must be positive integer
   - All fields required

4. **Wall-clock semantics display**
   - Clear explanation that times are wall-clock in selected timezone
   - Warning about DST implications if applicable
   - No elapsed-duration calculations shown

5. **Lifecycle awareness**
   - Show when schedule is locked (post-bidding commencement)
   - Display clear indicators when editing blocked
   - Preserve read-only access when locked

### B. Form Fields

#### Timezone Selection

- UI element: Searchable dropdown or autocomplete
- Data source: IANA timezone database
- Common timezones highlighted (e.g., US timezones)
- Display format: "America/New_York (Eastern Time)"
- Validation: Must be valid IANA identifier

Implementation note: Consider using a timezone picker library or
curated list of common timezones to avoid overwhelming users.

#### Start Date Selection

- UI element: Date picker
- Constraints:
  - Must be a Monday (validation error if not)
  - Should be in the future (warning if not, hard error at confirmation)
  - Display day-of-week clearly
- Format: YYYY-MM-DD (ISO 8601 date-only)

#### Daily Bid Window

- Start time: Time picker (HH:MM format, 24-hour or 12-hour)
- End time: Time picker (HH:MM format, 24-hour or 12-hour)
- Validation: End must be after start
- Display: Wall-clock times in selected timezone
- Help text: "Bidding window applies every day during the bid period"

Note: Do NOT support multi-day windows. If end time is before start time,
this is a validation error (not a day-spanning window).

#### Bidders Per Area Per Day

- UI element: Number input
- Validation: Must be positive integer (≥ 1)
- Help text: "Number of users who may bid per area per day"
- Default suggestion: 10 (non-binding)

### C. Backend API Integration

Add to `ui/src/api.ts`:

```typescript
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

Add corresponding types to `ui/src/types.ts`:

- `BidScheduleInfo`
- `SetBidScheduleRequest`
- `SetBidScheduleResponse`
- `GetBidScheduleRequest`
- `GetBidScheduleResponse`

### D. Integration with Bootstrap Workflow

If part of 30D restructure:

- BidScheduleSetup is a dedicated section in bootstrap workflow
- Route: `/admin/bootstrap/schedule`
- Positioned after Area Round Assignment
- Before Readiness Review

If standalone:

- Add route in `ui/src/App.tsx`
- Update navigation to include bid schedule link

### E. Timezone Handling

**Critical constraint:** All times are **wall-clock times** in the selected timezone.

UI must NOT:

- Imply duration semantics
- Convert to UTC for display
- Hide timezone from user
- Suggest elapsed time across DST boundaries

UI must:

- Display timezone prominently
- Show times exactly as entered
- Explain DST implications if applicable (info text, not automatic adjustment)

**Example display:**

```text
Bid Schedule
Timezone: America/New_York (Eastern Time)
Start Date: 2026-03-09 (Monday)
Daily Window: 08:00 - 17:00 (wall-clock)
Bidders Per Day: 10 per area

Note: Times are wall-clock in the selected timezone.
During Daylight Saving Time transitions, the window
remains 08:00-17:00 local time each day.
```

### F. Validation Error Messages

- Timezone invalid: "Please select a valid timezone"
- Start date not Monday: "Start date must be a Monday"
- Start date in past: "Start date should be in the future (warning at save, error at confirmation)"
- Window end before start: "Daily window end time must be after start time"
- Bidders invalid: "Bidders per day must be a positive integer"

### G. Styling

Create or extend SCSS module:

- `ui/src/styles/bid-schedule-setup.module.scss`

Follow AGENTS.md styling guidelines:

- Mobile-first responsive design
- Form fields full-width on mobile
- Clear labels and help text
- Time pickers touch-friendly
- No inline styles
- Use existing design tokens

---

## UI Design Constraints

### Mobile-First

- Form: single column layout
- Date/time pickers: native mobile-friendly controls preferred
- Timezone picker: searchable on mobile
- Help text: collapsible on mobile if lengthy

### Lifecycle Awareness

- Show lifecycle state badge
- Disable form when locked
- Clear "locked" indicator
- Preserve read-only display

### Readiness Integration

- If schedule not set, show as blocker
- Clear indication when complete
- Link from readiness review if incomplete

### Error Handling

- Validation errors: inline, per-field
- Backend rejections: surface structured errors
- Network errors: retry-friendly messages
- DST warnings: informational, not blocking

---

## Validation & Testing

### Manual Validation

After implementation:

1. Set bid schedule with all fields
2. Edit bid schedule
3. Verify timezone selection works
4. Test start date validation (non-Monday should error)
5. Test daily window validation (end before start should error)
6. Verify bidders per day accepts positive integers only
7. Test lifecycle lock (after bidding commences)
8. Verify mobile responsiveness
9. Test with various timezones (especially DST-observing)

### Constraint Validation

- Start date Monday constraint enforced
- Daily window start < end enforced
- Bidders per day > 0 enforced
- All fields required

### Edge Cases

- Timezone with DST transition during bid period
- Start date validation at confirmation time (if in past)
- Very early or very late daily windows (e.g., 00:00-23:59)

---

## Backend Endpoint Verification

Before implementation, verify:

1. `POST /bid-schedule` or equivalent exists
2. `GET /bid-schedule?bid_year_id=<id>` or equivalent exists
3. Backend validates all constraints
4. Backend enforces lifecycle locks
5. Backend stores timezone as IANA identifier
6. Backend stores times as wall-clock (HH:MM:SS format)

If any endpoint is missing or semantics differ, **stop and document the gap**.

---

## Explicit Non-Goals

- No DST automatic adjustments
- No duration calculations
- No bid window preview (deferred to bid order preview)
- No multi-timezone support (single timezone per bid year)
- No backend implementation
- No domain logic changes

---

## Completion Conditions

This sub-phase is complete when:

- BidScheduleSetup component exists and renders
- All form fields functional and validated
- Timezone selection works correctly
- Backend API integration complete
- Frontend API bindings complete
- Lifecycle constraints enforced in UI
- Wall-clock semantics clearly communicated
- Mobile usability verified
- Manual validation passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- All new files added via `git add`
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Backend bid schedule API does not exist
- API semantics conflict with wall-clock requirements
- Timezone handling is ambiguous
- DST semantics are unclear
- Start date validation rules conflict with domain invariants
- Lifecycle enforcement missing in backend

---

## Risk Notes

- Timezone handling is complex and error-prone
- DST semantics must be clearly communicated, not automated
- Start date validation at confirmation time may require backend support
- Wall-clock vs elapsed-time distinction is critical
- May require timezone library for IANA database
