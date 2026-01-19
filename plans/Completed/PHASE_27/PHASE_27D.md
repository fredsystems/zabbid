# Phase 27D — Ignored Test Enumeration

## Purpose

Create a complete inventory of ignored tests with categorization to enable informed remediation decisions.

## Scope

### Analysis Tasks

- Execute: `grep -r "#\[ignore\]" --include="*.rs"` across entire workspace
- For each ignored test found, determine:
  - File path and test name
  - Reason for ignore (if documented in comments)
  - Whether test is obsolete, integration-only, or legitimately deferred
  - Whether test could now be hermetic
  - Whether test requires external infrastructure (databases, services)

### Categorization Schema

Each ignored test must be categorized as one of:

- **Obsolete**: Test for removed functionality or superseded by other tests
- **Integration**: Requires external infrastructure (should be gated via xtask)
- **Hermetic-Candidate**: Was ignored for flakiness but could now be fixed
- **Justified**: Legitimately deferred with documented rationale in code (e.g. long-running performance tests or tests blocked by an explicitly planned future phase)
- If the reason for `#[ignore]` is not documented in code comments, the catalog must explicitly say “Reason undocumented” — no inferred justification.

### Output Artifact

Create `IGNORED_TESTS.md` catalog with:

- Test location (file path, line number)
- Test name
- Current status category
- Recommendation (remove / gate via xtask / unignore / keep)
- Notes about dependencies, risks, or context
- Any comments from source code explaining why ignored

## Explicit Non-Goals

- Do NOT remove any ignored tests in this phase
- Do NOT unignore any tests in this phase
- Do NOT fix tests in this phase
- Do NOT create xtask runners in this phase
- Do NOT run the ignored tests to determine behavior

## Files Likely to Be Analyzed

All test files across workspace:

- `crates/*/tests/*.rs`
- `crates/*/src/**/tests.rs`
- `crates/*/src/**/*.rs` (inline `#[cfg(test)]` modules)

## Search Patterns

Execute the following searches:

```bash
# Find all ignored tests with context
grep -rn "#\[ignore\]" --include="*.rs" -A 2 -B 2

# Find ignored tests with reasons in comments
grep -rn "#\[ignore\]" --include="*.rs" -B 5 | grep -E "//|/\*"

# Count ignored tests by crate
find crates -name "*.rs" -exec grep -l "#\[ignore\]" {} \; | cut -d/ -f2 | sort | uniq -c
```

## Example Catalog Entry

```markdown
### Test: `test_mariadb_migration_parity`

- **Location**: `crates/persistence/tests/migrations.rs:145`
- **Category**: Integration
- **Recommendation**: Gate via `cargo xtask test-mariadb`
- **Reason**: Requires MariaDB container, cannot run in default test suite
- **Notes**: Test is valid and should run in CI, just not in `cargo test`
```

## Completion Conditions

- `IGNORED_TESTS.md` created with complete inventory
- Every ignored test has a categorization
- Recommendations are clear and actionable
- Document passes markdown linting (`cargo xtask ci`)
- Document passes pre-commit hooks
- Git commit contains only the catalog document

## Dependencies

None (can run in parallel with 27B, 27C, 27E, 27I)

## Blocks

- Phase 27F (Test Isolation and Determinism Fixes)

Cannot remediate ignored tests without knowing what they are and why they're ignored.

## Execution Notes

### Analysis Approach

For each ignored test:

1. Read surrounding code and comments
2. Identify what the test is testing
3. Check if functionality still exists
4. Determine why it was ignored (if documented)
5. Assess whether infrastructure is now available
6. Categorize and recommend

### Common Ignore Reasons

- External database required (MariaDB, PostgreSQL)
- Flaky due to timing or randomness
- Feature not yet implemented
- Performance test (slow, not for normal runs)
- Temporarily broken during refactor

### Documentation Quality

The catalog must be detailed enough that Phase 27F can execute remediation without re-analyzing each test.

Include enough context that someone unfamiliar with the test can understand:

- What it tests
- Why it matters
- What's blocking it from running
- What would be required to enable it

## Summary

| Category           | Count |
| ------------------ | ----- |
| Obsolete           | 3     |
| Integration        | 5     |
| Hermetic-Candidate | 2     |
| Justified          | 1     |
