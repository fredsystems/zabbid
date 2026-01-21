# Phase 30H — End-to-End Validation

## Purpose

Perform comprehensive end-to-end validation of the complete Phase 30 UI
implementation, ensuring that a real operator can complete the entire
pre-bid workflow from empty system to "Ready to Bid" using only the UI.

This sub-phase delivers the **operational proof** that Phase 30 objectives
have been met and the system is ready for real-world use.

---

## Scope

### A. End-to-End Workflow Validation

Execute a complete bootstrap workflow using only the UI:

1. **Initial Setup**
   - Log in as admin operator
   - Verify bootstrap mode or create first admin
   - Confirm authenticated state

2. **Bid Year Creation & Activation**
   - Create a new bid year
   - Set expected area count
   - Set bid year as active
   - Verify bid year appears in all relevant UI sections

3. **Area Configuration**
   - Create all required areas (competitive areas only at this stage)
   - Set expected user count per area
   - Verify No Bid system area exists
   - Verify area count matches expected

4. **User Import & Management**
   - Import users via CSV (200+ users recommended)
   - Preview CSV import
   - Confirm import
   - Manually create 2-3 additional users
   - Edit user metadata (name, seniority dates)
   - Verify user counts per area
   - Set user participation flags (exclude 2-3 users from bidding)
   - Verify participation constraint enforcement

5. **No Bid Review**
   - Navigate to No Bid review section
   - If users in No Bid: reassign all to competitive areas
   - Verify zero users remain in No Bid
   - Verify blocker clears from readiness status

6. **Round Groups & Rounds Setup**
   - Create 2-3 round groups
   - Add rounds to each group (e.g., 5 rounds per group)
   - Edit round names and numbers
   - Verify round structure

7. **Area → Round Group Assignment**
   - Assign round group to each non-system area
   - Verify all areas have assignments
   - Verify blocker clears from readiness status

8. **Bid Schedule Configuration**
   - Select timezone (e.g., America/New_York)
   - Set bid start date (Monday, future date)
   - Set daily bid window (e.g., 08:00-17:00)
   - Set bidders per area per day (e.g., 10)
   - Verify schedule is saved
   - Verify blocker clears from readiness status

9. **Bid Order Preview**
   - View bid order for each area
   - Verify users are ordered correctly
   - Check tie-breaker data is visible
   - Verify excluded users appear appropriately
   - Check for seniority conflicts (should be none if data is clean)

10. **Readiness Review & Confirmation**
    - Navigate to readiness review section
    - Verify no blockers remain
    - Verify "Ready to Bid" status displayed
    - Review configuration summary
    - Open confirmation modal
    - Enter confirmation phrase: "I understand this action is irreversible"
    - Submit confirmation
    - Verify success message
    - Verify lifecycle state transitions to Canonicalized

11. **Post-Confirmation Verification**
    - Attempt to edit user (should be locked)
    - Attempt to edit area (should be locked)
    - Attempt to edit round group (should be locked)
    - Attempt to edit bid schedule (should be locked)
    - Verify bid order shows "Frozen" status
    - Verify all UI sections show locked state
    - Verify read-only access preserved

### B. Mobile Responsiveness Validation

Repeat critical workflow steps on mobile viewport:

1. **Test viewport sizes:**
   - 320px width (small phone)
   - 375px width (iPhone)
   - 768px width (tablet)

2. **Critical mobile paths:**
   - Create bid year
   - Import CSV users
   - Edit user participation flags
   - Assign round groups to areas
   - Set bid schedule
   - Review readiness and confirm

3. **Mobile-specific checks:**
   - All controls touch-friendly
   - No horizontal scrolling
   - Text readable without zooming
   - Forms usable in portrait orientation
   - Navigation accessible
   - Modals render correctly

### C. Error Handling Validation

Test error scenarios:

1. **Network errors:**
   - Disconnect network during operation
   - Verify retry-friendly error messages
   - Reconnect and retry
   - Verify operation completes

2. **Validation errors:**
   - Submit incomplete forms
   - Verify inline validation errors
   - Enter invalid data (e.g., non-Monday start date)
   - Verify clear error messages

3. **Lifecycle constraint violations:**
   - After confirmation, attempt locked operations
   - Verify clear "locked" messaging
   - Verify no silent failures

4. **Concurrent operations:**
   - Open two browser windows
   - Perform conflicting operations
   - Verify one succeeds, one errors appropriately
   - Verify live event updates both windows

### D. Performance Validation

Test with realistic data volumes:

1. **Large user datasets:**
   - 200+ users across 5+ areas
   - CSV import performance acceptable (< 10 seconds)
   - User list rendering acceptable (< 2 seconds)
   - Pagination or virtualization if needed

2. **Complex round structures:**
   - 3+ round groups with 10+ rounds each
   - Round list rendering acceptable

3. **Bid order computation:**
   - Bid order for area with 50+ users
   - Preview loads in acceptable time (< 5 seconds)
   - Tie-breaker data displays without lag

### E. Browser Compatibility Validation

Test in multiple browsers:

1. **Required browsers:**
   - Chrome/Edge (latest)
   - Firefox (latest)
   - Safari (latest, desktop and iOS)

2. **Validation per browser:**
   - All critical workflows complete successfully
   - UI renders correctly
   - No console errors
   - Forms submit correctly

### F. Accessibility Validation (Basic)

Basic accessibility checks:

1. **Keyboard navigation:**
   - Tab through forms
   - Submit with Enter key
   - Cancel with Escape key

2. **Screen reader compatibility (basic):**
   - Form labels associated with inputs
   - Error messages announced
   - Buttons have descriptive labels

3. **Color contrast:**
   - Text readable against backgrounds
   - Status indicators distinguishable

---

## Validation Deliverables

### A. Validation Report

Create: `plans/PHASE_30/END_TO_END_VALIDATION_REPORT.md`

This document must contain:

1. **Test Environment**
   - Date and time of testing
   - Browser(s) used
   - Dataset size (user count, area count, etc.)

2. **Workflow Results**
   - Each workflow step: ✅ Pass | ❌ Fail
   - Any deviations or issues encountered
   - Screenshots or recordings (if applicable)

3. **Mobile Validation Results**
   - Viewport sizes tested
   - Critical issues found
   - Pass/fail per viewport

4. **Error Handling Results**
   - Error scenarios tested
   - Expected vs actual behavior
   - Any silent failures or unclear messaging

5. **Performance Results**
   - Operation timings (CSV import, page loads, etc.)
   - Any performance degradation areas
   - Acceptable: Yes | No

6. **Browser Compatibility Results**
   - Matrix of browsers × workflows
   - Any browser-specific issues

7. **Issues Found**
   - List of all bugs, inconsistencies, or UX problems
   - Severity: Critical | Major | Minor
   - Blocking Phase 30 completion: Yes | No

8. **Overall Assessment**
   - Can the workflow be completed end-to-end? Yes | No
   - Are Phase 30 objectives met? Yes | No
   - Recommended actions before phase completion

### B. Bug Fixes (If Required)

If validation uncovers bugs:

1. **Critical bugs:**
   - Fix immediately before completing Phase 30
   - Re-run validation to confirm fix

2. **Major bugs:**
   - Fix if feasible within Phase 30 scope
   - Otherwise, document for follow-up

3. **Minor bugs:**
   - Document for future phases
   - Do not block Phase 30 completion

---

## Validation Data Setup

### Recommended Test Dataset

1. **Bid Year:**
   - Year: 2026
   - Start date: 2026-01-05 (example Monday)
   - Pay periods: 26
   - Expected areas: 5

2. **Areas:**
   - Area 1: "D10" (15 users expected)
   - Area 2: "D21" (20 users expected)
   - Area 3: "TMU" (10 users expected)
   - Area 4: "ADMIN" (5 users expected)
   - Area 5: "TRAINING" (5 users expected)
   - System area: "NO BID" (0 users expected, auto-created)

3. **Users:**
   - Total: 55 users (matching expected counts)
   - Prepare CSV with realistic data:
     - Unique initials
     - Realistic seniority dates
     - Mix of user types (CPC, Trainee, Admin)
     - Crew assignments (1-7)
   - 2-3 users initially in No Bid (to test review workflow)
   - 2-3 users excluded from bidding

4. **Round Groups:**
   - Group 1: "Standard Rounds" (5 rounds)
   - Group 2: "Short Rounds" (3 rounds)
   - Group 3: "Extended Rounds" (7 rounds)

5. **Round Group Assignments:**
   - D10 → Standard Rounds
   - D21 → Standard Rounds
   - TMU → Short Rounds
   - ADMIN → Short Rounds
   - TRAINING → Extended Rounds

6. **Bid Schedule:**
   - Timezone: America/New_York
   - Start date: Next Monday after today
   - Daily window: 08:00-17:00
   - Bidders per area per day: 10

---

## Explicit Non-Goals

- No automated UI testing (manual validation only)
- No load testing or stress testing
- No security testing
- No production deployment validation
- No code changes beyond bug fixes

---

## Completion Conditions

This sub-phase is complete when:

- End-to-end validation executed with 200+ users
- Validation report completed and committed
- All critical bugs fixed and re-validated
- Major bugs either fixed or documented
- Mobile validation passed on all target viewport sizes
- Browser compatibility validated on all required browsers
- Phase 30 objectives confirmed met
- User approves validation results

---

## Stop-and-Ask Conditions

Stop immediately if:

- Critical workflow failure prevents end-to-end completion
- Data corruption or loss observed
- Lifecycle transitions fail or behave unexpectedly
- Confirmation action is reversible (violates irreversibility)
- Multiple critical bugs found (indicates deeper issues)
- Performance is unacceptable with realistic data

If any of these occur, validation has **failed** and Phase 30 cannot be
completed until issues are resolved.

---

## Risk Notes

- End-to-end validation may uncover integration issues missed in unit testing
- Real-world data may expose edge cases not anticipated
- Mobile testing may reveal layout issues not visible on desktop
- Browser compatibility issues may require significant refactoring
- Performance issues with large datasets may require optimization
- This is the final gate before Phase 30 completion — failure here is significant
