# Phase 27 — Planning Summary

## Planning Complete

Phase 27 has been fully decomposed into 11 independently executable sub-phases.

All planning documents have been created and pass linting checks:

- `cargo xtask ci` — ✅ Passed
- `pre-commit run --all-files` — ✅ Passed

## Planning Documents Created

### Core Planning

- `README.md` — Phase overview and execution guide
- `PLANNING_SUMMARY.md` — This document

### Sub-Phase Plans

- `PHASE_27A.md` — AGENTS.md Audit and Clarification
- `PHASE_27B.md` — User Identity Correctness Verification
- `PHASE_27C.md` — Area and Bid Year Identity Audit
- `PHASE_27D.md` — Ignored Test Enumeration
- `PHASE_27E.md` — Flaky Test Root Cause Analysis
- `PHASE_27F.md` — Test Isolation and Determinism Fixes
- `PHASE_27G.md` — Coverage Measurement and Gap Identification
- `PHASE_27H.md` — Coverage Gap Remediation
- `PHASE_27I.md` — UI Styling Audit and Violation Catalog
- `PHASE_27J.md` — UI Precision Fixes
- `PHASE_27K.md` — Tooling Cleanup

## Critical Findings from Planning Analysis

### Ambiguities Requiring Resolution

The following ambiguities were identified during planning analysis and MUST be resolved before execution can begin:

#### 1. Area Identity Model (Blocks 27A, 27B, 27C)

**Question**: Do areas use canonical `area_id` (numeric) vs display `area_code` (string) pattern?

**Evidence**:

- AGENTS.md documents user identity exhaustively (L213-223)
- AGENTS.md has NO equivalent section for areas
- Code shows `area_id` usage in CSV preview as strings
- Unclear if areas follow same canonical identity pattern as users

**Required for**: Phase 27A documentation updates, Phase 27C identity audit

#### 2. Bid Year Identity Model (Blocks 27A, 27B, 27C)

**Question**: Do bid years use canonical `bid_year_id` (numeric) vs display `year` (integer value) pattern?

**Evidence**:

- AGENTS.md documents bid years exist (L487-492) but NOT identity semantics
- Phase 26 likely introduced canonical bid year tables
- Unclear if year value is identifier or display metadata

**Required for**: Phase 27A documentation updates, Phase 27C identity audit

#### 3. Phase 23A Completion Status (Blocks 27A)

**Question**: Is Phase 23A (Canonical Identity for Area & Bid Year) complete?

**Evidence**:

- AGENTS.md contains "TODO — Post Phase 23A Enforcement" section (L760-804)
- Rules reference "after Phase 23A" without stating current applicability
- Unclear if these rules are NOW active or still pending

**Required for**: Phase 27A AGENTS.md cleanup

#### 4. Override and Edit Semantics (Blocks 27A)

**Question**: When canonical data is edited, what audit trail is required? What lifecycle constraints apply?

**Evidence**:

- Phase 26 likely introduced edit capabilities
- AGENTS.md has NO documented rules for overrides
- No audit trail requirements for edits documented

**Required for**: Phase 27A documentation additions

## Execution Prerequisites

### User Must Resolve Before Starting

Before Phase 27A can execute, user must clarify:

1. Area identity model (canonical ID vs area code)
2. Bid year identity model (canonical ID vs year value)
3. Phase 23A completion status
4. Override and edit audit requirements

### Environment Requirements

Verify before execution:

- `llvm-cov` available (required for Phase 27G)
- `pre-commit` hooks installed and functional
- `cargo xtask ci` operational
- Docker available (for MariaDB tests)

### Baseline Requirements

Confirm before execution:

- Main branch builds successfully
- Test suite passes (even with ignored tests)
- UI renders without console errors

## Recommended Execution Approach

### Phase 1: Resolve Ambiguities

**User action required**: Clarify the four critical ambiguities listed above

### Phase 2: Documentation Foundation

**Execute**: Phase 27A — Update AGENTS.md with clarified rules

**Blocks**: All identity-related work (27B, 27C)

### Phase 3: Parallel Analysis Workstreams

**Execute in parallel**:

- Phase 27B — User identity verification
- Phase 27C — Area/bid year identity verification
- Phase 27D — Ignored test enumeration
- Phase 27E — Flaky test analysis
- Phase 27I — UI violation catalog
- Phase 27K — Tooling cleanup

### Phase 4: Test Reliability

**Execute**: Phase 27F — Fix test reliability issues

**Depends on**: 27D (inventory), 27E (analysis)

**Blocks**: 27G (coverage requires reliable tests)

### Phase 5: Coverage Work

**Execute**: Phase 27G — Measure coverage and identify gaps

**Depends on**: 27F (reliable tests)

**Blocks**: 27H (gap remediation)

### Phase 6: Final Implementations

**Execute in parallel**:

- Phase 27H — Add missing tests
- Phase 27J — Fix UI violations

**Depends on**: 27G (coverage gaps), 27I (UI violations)

## Context and Token Management

Each sub-phase is designed to:

- Focus on a single concern or module
- Avoid whole-codebase analysis requirements
- Fit within agent context limits
- Produce small, reviewable commits

If any phase appears to exceed context limits during execution, it should be split further with user guidance.

## Success Metrics

Phase 27 execution succeeds when:

- ✅ AGENTS.md aligned with implementation
- ✅ Identity semantics enforced (user, area, bid year)
- ✅ Test suite deterministic (zero flaky tests)
- ✅ Coverage measured and critical gaps filled
- ✅ UI compliant with styling rules
- ✅ Dead tooling removed

## Planning Quality Verification

This planning phase succeeds if:

- ✅ Each sub-phase can be executed independently
- ✅ No sub-phase requires inventing domain rules
- ✅ No sub-phase requires guessing intent
- ✅ Execution order is clear and dependencies documented
- ✅ Context limits are respected through decomposition
- ✅ All planning documents pass linting

**Status**: All criteria met ✅

## Next Steps

1. **User review**: Review planning documents and resolve ambiguities
2. **User approval**: Approve execution to begin
3. **Execute 27A**: Start with AGENTS.md audit after ambiguities resolved
4. **Sequential execution**: Follow recommended execution approach above

## Planning Artifacts Location

All planning documents are in: `plans/phase_27/`

- Entry point: `README.md`
- Individual phase plans: `PHASE_27A.md` through `PHASE_27K.md`
- This summary: `PLANNING_SUMMARY.md`

## Planning Phase Complete

Ready for user review and execution authorization.
