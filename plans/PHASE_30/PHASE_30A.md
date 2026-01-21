# Phase 30A — Phase 29 Gap Analysis

## Purpose

Before implementing UI for Phase 29 features, we must verify that all
required Phase 29 backend capabilities exist and are operable.

This sub-phase performs a **systematic audit** of Phase 29 deliverables
against Phase 30 requirements, identifies gaps, and documents them
explicitly for user review.

This is a **read-only, analysis-only** sub-phase. No code changes are permitted.

---

## Scope

### A. API Surface Inventory

Enumerate all Phase 29 APIs by examining:

- `crates/api/src/handlers.rs` (all public functions)
- `crates/server/src/routes.rs` or equivalent routing logic
- `ui/src/api.ts` (frontend API client)

For each API endpoint, document:

- HTTP method and path
- Handler function name
- Purpose
- Request/response types
- Whether frontend bindings exist

### B. Required Capabilities Checklist

Verify the existence of APIs for:

1. **Round Groups**
   - Create round group
   - List round groups
   - Update round group
   - Delete round group

2. **Rounds**
   - Create round
   - List rounds (for a round group)
   - Update round
   - Delete round

3. **Area → Round Group Assignment**
   - Assign round group to area
   - Retrieve area's assigned round group
   - Validate exactly-one-per-non-system-area constraint

4. **Bid Schedule**
   - Set bid schedule (timezone, start date, window, bidders/day)
   - Get bid schedule
   - Validate DST-safe wall-clock semantics

5. **User Participation Flags**
   - Update `excluded_from_bidding`
   - Update `excluded_from_leave_calculation`
   - Enforce lifecycle constraints (immutable after confirmation)

6. **Readiness & Confirmation**
   - Get readiness status (blockers list)
   - Confirm ready to bid (irreversible transition)

7. **No Bid Review**
   - List users in No Bid area
   - Review/confirm user disposition
   - Reassign to competitive area

8. **Bid Order Preview**
   - Get bid order for an area (pre-confirm: derived)
   - Get bid order for an area (post-confirm: frozen)
   - Display tie-breaker inputs

9. **Bid Windows (if present)**
   - Get bid windows for users
   - Understand frozen vs derived semantics

### C. Gap Identification

For each missing or incomplete capability:

- Document what is missing
- Note whether it blocks Phase 30 execution
- Propose resolution (defer, implement, or workaround)

### D. Frontend API Bindings Audit

For each Phase 29 API that exists:

- Check if `ui/src/api.ts` has a corresponding function
- Check if request/response types are defined in `ui/src/types.ts`
- Document any missing bindings

---

## Deliverables

At the end of this sub-phase, produce a single markdown document:

**`plans/PHASE_30/PHASE_29_GAP_ANALYSIS.md`**

This document must contain:

1. **API Inventory Table**
   - Columns: Endpoint | Handler | Purpose | Frontend Binding Exists
   - All Phase 29 APIs listed

2. **Capability Coverage Matrix**
   - Rows: Required capabilities (from checklist above)
   - Columns: Backend Exists | Frontend Binding | Status
   - Status values: ✅ Complete | ⚠️ Partial | ❌ Missing

3. **Identified Gaps**
   - Explicit list of missing capabilities
   - Severity: Blocking | Non-Blocking
   - Recommended action

4. **Assumptions & Notes**
   - Document any ambiguities
   - Note any Phase 29 features that exceed Phase 30 scope
   - Explicitly note any UI flows that would require composite or orchestration APIs (multiple backend calls) rather than a single endpoint

---

## Explicit Non-Goals

- No code changes
- No API implementation
- No frontend binding creation
- No testing

---

## Completion Conditions

This sub-phase is complete when:

- API inventory is complete and accurate
- Capability coverage matrix is filled
- All gaps are documented with severity and recommendation
- Gap analysis document is committed
- User has reviewed and approved findings

---

## Stop-and-Ask Conditions

Stop immediately if:

- Phase 29 appears to have delivered features beyond documented scope
  (this may indicate scope drift)
- Critical blocking gaps are found that invalidate Phase 30 feasibility
- API semantics conflict with Phase 30 invariants
- Domain invariants appear to have been violated in Phase 29

---

## Risk Notes

- This sub-phase assumes Phase 29 is complete and correct
- If Phase 29 is incomplete, Phase 30 execution may need to pause
- Gap severity assessment is critical for planning remaining sub-phases
