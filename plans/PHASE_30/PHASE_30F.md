# Phase 30F — Readiness Review & Confirmation UI

## Purpose

Implement the UI for reviewing bid year readiness status, displaying all
blocking reasons, and performing the irreversible "Confirm Ready to Bid"
action that transitions the system to Canonicalized state.

This sub-phase delivers the **critical gate** between pre-bid configuration
and operational bidding, ensuring all prerequisites are met before freezing
canonical state.

---

## Scope

### A. Readiness Review Component

Create `ui/src/components/ReadinessReview.tsx` (if not already created in 30D).

This component must:

1. **Display overall readiness state**
   - Current lifecycle state badge
   - Readiness status: "Ready to Bid" or "Not Ready"
   - Visual indicator (✅ ready, ❌ blocked)
   - Last checked timestamp

2. **List all blocking reasons** (if not ready)
   - Structured display of each blocker
   - Clear description of what is incomplete
   - Direct link to resolve (jump to relevant section)
   - Priority/severity indication if applicable

3. **Show readiness summary** (when ready)
   - Summary of configuration:
     - Bid year and active status
     - Area count (expected vs actual)
     - User count (total, by area if useful)
     - Round groups configured
     - Bid schedule set
     - No Bid users resolved
   - Confirmation that all gates passed

4. **Confirm Ready to Bid action**
   - Prominent "Confirm Ready to Bid" button
   - Only enabled when no blockers remain
   - Disabled when already confirmed (lifecycle = Canonicalized or later)
   - Triggers confirmation modal

5. **Refresh readiness status**
   - Manual refresh button
   - Auto-refresh on live events (if applicable)
   - Loading state during refresh

### B. Confirmation Modal

Create a modal dialog for the irreversible confirmation action.

The modal must:

1. **Display critical information**
   - Bid year being confirmed
   - Current configuration summary
   - Irreversibility warning (bold, prominent)

2. **Require explicit acknowledgment**
   - Text input field requiring exact phrase:
     `"I understand this action is irreversible"`
   - Submit button disabled until phrase matches exactly
   - Case-sensitive match required

3. **Show consequences**
   - "After confirmation, the following will be locked:"
     - User roster (no adds, deletes, or area reassignments)
     - Area structure (no adds, deletes)
     - Round groups and rounds (no modifications)
     - Bid schedule (immutable until bidding commences)
     - Bid order frozen to canonical state
   - "Bid windows will be calculated and frozen"
   - "No Bid users must be resolved before confirmation"

4. **Submission and feedback**
   - Clear "Confirm" button (disabled until acknowledgment)
   - "Cancel" button (always enabled)
   - Loading state during submission
   - Success message on completion
   - Error handling with retry option

### C. Blocking Reasons Display

For each blocking reason, display:

1. **Blocker type**
   - Area count mismatch
   - User count mismatch (per area)
   - No Bid users unresolved
   - Round group not assigned to area
   - Bid schedule not set
   - Seniority conflicts (if applicable)
   - Custom blockers from backend

2. **Blocker details**
   - Affected entity (bid year, area, user)
   - Expected vs actual values
   - Specific constraint violated

3. **Resolution link**
   - Direct navigation to section that can resolve blocker
   - Example: "Go to Area Setup" or "Review No Bid Users"

4. **Visual hierarchy**
   - Critical blockers: red, prominent
   - Warnings: yellow (if applicable)
   - Group by category (area-related, user-related, config-related)

### D. Backend API Integration

Add to `ui/src/api.ts`:

```typescript
export async function getBidYearReadiness(
  sessionToken: string,
  bidYearId: number,
): Promise<GetBidYearReadinessResponse>;

export async function confirmReadyToBid(
  sessionToken: string,
  bidYearId: number,
  confirmationPhrase: string,
): Promise<ConfirmReadyToBidResponse>;
```

Add corresponding types to `ui/src/types.ts`:

- `BidYearReadinessInfo`
- `BlockingReason` (with discriminated union for types)
- `GetBidYearReadinessResponse`
- `ConfirmReadyToBidRequest`
- `ConfirmReadyToBidResponse`

### E. Integration with Bootstrap Workflow

If part of 30D restructure:

- ReadinessReview is the final section in bootstrap workflow
- Route: `/admin/bootstrap/readiness`
- Positioned after Bid Schedule
- Terminal section (no "Next" button after confirmation)

If standalone:

- Add route in `ui/src/App.tsx`
- Update navigation to include readiness review link
- Ensure accessible from all bootstrap sections (readiness widget)

### F. Post-Confirmation Behavior

After successful confirmation:

1. **Lifecycle state update**
   - Bid year transitions to Canonicalized
   - UI reflects new state across all components
   - Live event triggers refresh (if applicable)

2. **Navigation**
   - Remain on readiness review page
   - Show success message
   - Display "Confirmed" status with timestamp
   - Disable confirmation button
   - Show link to next workflow (if applicable)

3. **UI updates**
   - All bootstrap sections show locked state
   - Edit controls disabled across all components
   - Read-only access preserved

### G. Styling

Create or extend SCSS module:

- `ui/src/styles/readiness-review.module.scss`

Follow AGENTS.md styling guidelines:

- Mobile-first responsive design
- Clear status indicators
- Prominent confirmation button when ready
- Clear blocker display with visual hierarchy
- Modal overlay for confirmation
- No inline styles
- Use existing design tokens

Key visual elements:

- Ready state: Green badge, large checkmark
- Blocked state: Red badge, warning icon
- Blockers: Card-based list, expandable details
- Confirmation button: Large, prominent, color-coded (green when enabled)
- Modal: Centered overlay, clear focus on warning text

---

## UI Design Constraints

### Mobile-First

- Readiness summary: stacked vertical layout
- Blockers list: cards, one per row
- Confirmation modal: full-screen on mobile, centered on desktop
- Resolution links: touch-friendly buttons
- All text readable without zooming

### Irreversibility Communication

- **Critical:** Irreversibility must be impossible to miss
- Use bold text, warning colors, explicit language
- Confirmation phrase prevents accidental clicks
- Modal provides final "are you sure" gate

### Lifecycle Awareness

- Show current lifecycle state prominently
- When Canonicalized: show "Already Confirmed" state
- When Bidding_Active or later: show bid status (out of scope for now)
- Disable confirmation when not in Bootstrap_Complete

### Readiness Widget Integration

- If ReadinessWidget exists (from 30D):
  - Sync status between widget and review page
  - Widget links to this component
  - Both update on live events

### Error Handling

- Network errors: retry-friendly messages
- Validation errors: explain what's wrong (inline)
- Backend rejections: surface structured errors
- Confirmation phrase mismatch: clear inline error

---

## Validation & Testing

### Manual Validation

After implementation:

1. View readiness with blockers present
2. Verify blocker descriptions are clear
3. Follow resolution links to relevant sections
4. Resolve all blockers
5. Verify "Ready to Bid" state displays
6. Test confirmation modal:
   - Verify phrase validation (case-sensitive, exact match)
   - Test cancel button
   - Test submit with incorrect phrase (should error)
   - Test submit with correct phrase (should succeed)
7. Verify lifecycle transition to Canonicalized
8. Verify UI updates across all components
9. Attempt re-confirmation (should be disabled)
10. Test mobile responsiveness

### Constraint Validation

- Confirmation disabled when blockers present
- Confirmation phrase must match exactly
- Backend validates readiness server-side
- Lifecycle state updates correctly
- UI locks engage across all components

### Edge Cases

- Readiness changes between load and confirmation (race condition)
- Network failure during confirmation (retry behavior)
- Concurrent operator confirmation attempts
- Blocker appears after readiness check passes

---

## Backend Endpoint Verification

Before implementation, verify:

1. `GET /bid-years/:id/readiness` or equivalent exists
2. `POST /bid-years/:id/confirm-ready` or equivalent exists
3. Backend returns structured blocking reasons
4. Backend validates confirmation phrase server-side
5. Backend enforces all readiness constraints
6. Backend transitions lifecycle state atomically
7. Backend generates audit events for confirmation

If any endpoint is missing or semantics differ, **stop and document the gap**.

---

## Explicit Non-Goals

- No automatic readiness checking (manual refresh only, or live events)
- No blocker auto-resolution
- No partial confirmations
- No confirmation reversal (irreversible by design)
- No backend implementation
- No domain logic changes

---

## Completion Conditions

This sub-phase is complete when:

- ReadinessReview component exists and renders
- Blocker display functional and clear
- Confirmation modal implemented with phrase validation
- Backend API integration complete
- Frontend API bindings complete
- Post-confirmation behavior correct (lifecycle transition)
- Resolution links navigate correctly
- Irreversibility clearly communicated
- Mobile usability verified
- Manual validation passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- All new files added via `git add`
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Backend readiness API does not exist
- Backend confirmation API does not exist
- Blocker structure is ambiguous or inconsistent
- Confirmation phrase requirement differs from backend
- Lifecycle transition semantics unclear
- Readiness constraints conflict with domain invariants
- Backend does not enforce readiness server-side

---

## Risk Notes

- This is a critical, irreversible action
- Confirmation UX must be clear and deliberate
- Backend validation is authoritative (frontend is UX only)
- Lifecycle state must update atomically
- UI lock propagation must be reliable
- Testing with real data (200+ users) is essential
- Race conditions between readiness check and confirmation must be handled
