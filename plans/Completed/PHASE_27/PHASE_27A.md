# Phase 27A — AGENTS.md Audit and Clarification

## Purpose

Ensure AGENTS.md accurately documents the system as implemented and clarify ambiguities that block subsequent phases.

## Scope

### Analysis Tasks

- Review AGENTS.md sections L213-223 (Users) against actual user implementation
- Identify missing documentation for areas and bid years
- Assess whether "TODO — Post Phase 23A" (L760-804) is obsolete or active
- Identify where override/edit semantics need documentation
- Cross-reference UI styling rules (L591-600) against known violations

### Documentation Updates

- Add "Areas" subsection to Domain Invariants if pattern differs from users
- Add "Bid Years" subsection to Domain Invariants if pattern differs from users
- Add "Override & Edit Semantics" section if not covered elsewhere
- Remove or update "TODO — Post Phase 23A" based on actual completion status
- Clarify any rules agents have previously misinterpreted

## Explicit Non-Goals

- Do NOT add speculative rules for unimplemented features
- Do NOT weaken existing constraints
- Do NOT document implementation details (keep rules abstract)
- Do NOT add rules that would require code changes to comply

## Files Likely to Be Analyzed

- `AGENTS.md` (primary target)
- `crates/domain/src/user.rs`
- `crates/domain/src/area.rs`
- `crates/domain/src/bid_year.rs`
- `crates/persistence/src/canonical/users.rs`
- `crates/persistence/src/canonical/areas.rs`
- `crates/persistence/src/canonical/bid_years.rs`

## Critical Questions to Resolve

1. **Area Identity Model**: Do areas use canonical `area_id` (numeric) with `area_code` as display metadata, or is `area_code` itself the identifier?

2. **Bid Year Identity Model**: Do bid years use canonical `bid_year_id` (numeric) with `year` as display metadata, or is `year` itself the identifier?

3. **Phase 23A Status**: Is Phase 23A (Canonical Identity for Area & Bid Year) complete? Are the rules in the "TODO" section now active?

4. **Override Semantics**: When canonical data is edited, what audit trail is required? What lifecycle constraints apply?

## Completion Conditions

- AGENTS.md passes `cargo xtask ci` markdown linting
- AGENTS.md passes `pre-commit run --all-files` checks
- Area identity model documented unambiguously
- Bid year identity model documented unambiguously
- Override semantics documented (if applicable)
- No "TODO" sections with unclear applicability
- Git commit includes only AGENTS.md changes

## Dependencies

None (entry point for Phase 27)

## Blocks

- Phase 27B (User Identity Correctness Verification)
- Phase 27C (Area and Bid Year Identity Audit)

Cannot audit identity usage without clear identity rules.

## Execution Notes

This phase is documentation-only. No code changes are permitted.

If analysis reveals that code violates documented rules, note the violation but do not fix it in this phase. Identity correctness is addressed in 27B and 27C.

All changes must be confined to AGENTS.md to enable focused review.
