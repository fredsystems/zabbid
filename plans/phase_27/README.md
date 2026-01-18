# Phase 27 — Correctness, Audit, and Refinement Pass

## Overview

Phase 27 is a **system-wide correctness and hygiene pass**.
No new domain features are introduced.

The goal is to:

- Eliminate ambiguity in identity, lookup, and mutation semantics
- Make test coverage explicit, deterministic, and measurable
- Remove accumulated technical and behavioral debt
- Align code, tests, and AGENTS.md with the system as it exists today

This phase prioritizes **trustworthiness over velocity**.

## Phase Structure

Phase 27 is decomposed into 11 sub-phases to respect agent context limits and enable incremental review:

- **27A** — AGENTS.md Audit and Clarification
- **27B** — User Identity Correctness Verification
- **27C** — Area and Bid Year Identity Audit
- **27D** — Ignored Test Enumeration
- **27E** — Flaky Test Root Cause Analysis
- **27F** — Test Isolation and Determinism Fixes
- **27G** — Coverage Measurement and Gap Identification
- **27H** — Coverage Gap Remediation
- **27I** — UI Styling Audit and Violation Catalog
- **27J** — UI Precision Fixes
- **27K** — Tooling Cleanup

Each sub-phase is independently executable and reviewable.

## Execution Order

### Sequential Dependencies

```text
27A → 27B (identity rules required)
27A → 27C (identity rules required)
27D → 27F (inventory required)
27E → 27F (analysis required)
27F → 27G (reliable tests required)
27G → 27H (gap identification required)
27I → 27J (violation catalog required)
```

### Parallel Execution

After 27A completes, these can run in parallel:

- 27B, 27C, 27D, 27E, 27I, 27K

After 27F completes:

- 27G can start

After 27G and 27I complete:

- 27H and 27J can run in parallel

### Recommended Sequence

1. **27A** — Resolve documentation ambiguities first
2. **27B + 27C + 27D + 27E + 27I** — Parallel analysis workstreams
3. **27F** — Fix test reliability
4. **27G** — Measure coverage
5. **27H + 27J + 27K** — Parallel final work

## Sub-Phase Documents

Each sub-phase has a detailed planning document:

- `PHASE_27A.md` — AGENTS.md audit and clarification
- `PHASE_27B.md` — User identity correctness verification
- `PHASE_27C.md` — Area and bid year identity audit
- `PHASE_27D.md` — Ignored test enumeration
- `PHASE_27E.md` — Flaky test root cause analysis
- `PHASE_27F.md` — Test isolation and determinism fixes
- `PHASE_27G.md` — Coverage measurement and gap identification
- `PHASE_27H.md` — Coverage gap remediation
- `PHASE_27I.md` — UI styling audit and violation catalog
- `PHASE_27J.md` — UI precision fixes
- `PHASE_27K.md` — Tooling cleanup

Each document includes:

- Purpose and scope
- Analysis or implementation tasks
- Explicit non-goals
- Files likely affected
- Completion conditions
- Dependencies and blocking relationships

## Critical Pre-Execution Requirements

Before any sub-phase can execute, confirm:

### From 27A Execution

- Area identity model clearly documented
- Bid year identity model clearly documented
- Phase 23A completion status resolved
- Override semantics documented (if applicable)

### Environment

- llvm-cov available in toolchain (for 27G)
- pre-commit hooks installed and working
- xtask ci command operational

### Baseline

- Current main branch builds successfully
- Current test suite passes (even with ignored tests)
- Current UI renders without console errors

If baseline is broken, Phase 27 cannot proceed until fixed.

## Global Constraints

All sub-phases must adhere to these rules:

- Do NOT invent APIs or domain rules
- Do NOT silently change behavior
- Do NOT compensate for missing state
- Do NOT bypass domain rules for convenience
- Correctness takes precedence over completeness

If ambiguity is encountered, stop and ask before proceeding.

## Expected Artifacts

Phase 27 will produce:

### Documentation

- Updated `AGENTS.md` (from 27A)
- `IGNORED_TESTS.md` — Catalog of ignored tests (from 27D)
- `FLAKY_TESTS_ANALYSIS.md` — Root cause analysis (from 27E)
- `COVERAGE_GAPS.md` — Coverage gap catalog (from 27G)
- `UI_VIOLATIONS.md` — UI styling violation catalog (from 27I)

### Code Changes

- Identity correctness fixes (27B, 27C)
- Test reliability fixes (27F)
- New tests for coverage gaps (27H)
- UI styling fixes (27J)
- Tooling cleanup (27K)

### Metrics

- Coverage report (from 27G)
- Test reliability metrics (from 27F)
- Violation counts and resolutions (from 27I, 27J)

## Success Criteria

Phase 27 is complete when:

- AGENTS.md is accurate, enforced, and lint-clean
- Identity semantics are unambiguous and correct
- Test suite is deterministic and coverage-backed
- UI precision issues are resolved
- Tooling reflects actual supported usage
- No hidden correctness debt remains

## Design Philosophy

Phase 27 embodies these principles:

**Incremental Review**: Each sub-phase is small enough for focused review
**Context Awareness**: No sub-phase requires whole-codebase reasoning
**Explicit Dependencies**: Execution order is clear and documented
**Fail-Safe**: Agents stop on ambiguity rather than guessing
**Verifiable**: Each phase has concrete completion conditions

This structure prevents monolithic "fix everything" phases that exceed agent context limits and produce unreviewable PRs.
