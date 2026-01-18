# Phase 27G — Coverage Measurement and Gap Identification

## Purpose

Measure current test coverage and identify critical untested paths to enable targeted test additions.

## Scope

### Measurement Tasks

- Integrate llvm-cov if not already available: `cargo llvm-cov --html`
- Generate coverage report for entire workspace
- Analyze coverage metrics by module and crate
- Export coverage data in machine-readable format (lcov or json)

### Coverage Review Areas

Focus analysis on critical modules:

- `crates/core/src/` — State transitions and business logic
- `crates/api/src/handlers/` — Authorization and endpoint logic
- `crates/domain/src/` — Validation and invariants
- `crates/persistence/src/` — Database queries and canonical operations

### Gap Identification

Identify untested paths with high correctness risk:

- Authorization failure branches (admin-only, bidder-only endpoints)
- Validation error paths (domain rule violations)
- Lifecycle gating logic (operations on wrong-state entities)
- Canonicalization boundary checks (foreign key violations)
- Error handling paths (Result::Err branches)
- Edge cases in business logic (empty collections, boundary values)

### Output Artifact

Create `COVERAGE_GAPS.md` with:

- Current coverage percentage by crate
- Current coverage percentage by module
- List of critical untested paths:
  - File path and line range
  - Description of untested behavior
  - Why it matters (authorization, data integrity, audit, etc.)
  - Priority ranking (critical / high / medium / low)
  - Estimated test complexity (simple / moderate / complex)

## Explicit Non-Goals

- Do NOT write new tests in this phase (deferred to 27H)
- Do NOT aim for 100% coverage (focus on critical paths)
- Do NOT test trivial getters/setters or derived implementations
- Do NOT test third-party library code
- Do NOT prioritize coverage of dead code or unreachable paths

## Files Likely to Be Analyzed

### Coverage Report Output

- `target/llvm-cov/html/index.html` (generated report)
- `target/llvm-cov/lcov.info` (machine-readable data)

### Source Files for Gap Analysis

All production code in:

- `crates/core/src/**/*.rs`
- `crates/api/src/**/*.rs`
- `crates/domain/src/**/*.rs`
- `crates/persistence/src/**/*.rs`
- `crates/server/src/**/*.rs`

## Coverage Tooling

### Basic Coverage Generation

```bash
# Install if needed (should be in Nix environment)
cargo install cargo-llvm-cov

# Generate HTML coverage report
cargo llvm-cov --html --open

# Generate lcov format for analysis
cargo llvm-cov --lcov --output-path coverage.lcov
```

### Per-Crate Coverage

```bash
# Coverage for specific crate
cargo llvm-cov --package zabbid-core --html

# Coverage excluding integration tests
cargo llvm-cov --lib --html
```

## Priority Ranking Criteria

### Critical Priority

Must be tested, represents correctness or security risk:

- Authorization checks (who can perform action)
- Data integrity validation (domain invariants)
- Audit trail correctness (event recording)
- Foreign key constraint enforcement
- Last-admin protection logic
- State transition preconditions

### High Priority

Important but not immediate security/correctness risk:

- Error message formatting
- Lifecycle state validation
- Input sanitization
- Capability calculation logic
- CSV validation edge cases

### Medium Priority

Useful but lower risk:

- Error context enrichment
- Logging and instrumentation
- Display formatting
- Non-critical edge cases

### Low Priority

Nice to have but acceptable to defer:

- Performance optimizations
- Convenience methods
- Debug implementations
- Test-only code paths

## Expected Coverage Gaps

Based on typical development patterns, expect to find:

### Authorization Gaps

- Endpoints that don't test non-admin access
- Endpoints that don't test non-bidder access
- Missing tests for disabled operator attempts
- Missing tests for capability-based gating

### Validation Gaps

- Missing tests for invalid initials format
- Missing tests for duplicate detection
- Missing tests for lifecycle constraint violations
- Missing tests for malformed input data

### Error Path Gaps

- Happy path tested but error branches untested
- Database errors not simulated
- Validation failures not exercised
- Authorization failures not tested

### Edge Case Gaps

- Empty collections (no users, no areas)
- Boundary values (min/max dates, string lengths)
- Concurrent operations (race conditions)
- State transition ordering

## Analysis Approach

For each module with low coverage:

1. Review coverage report to identify uncovered lines
2. Determine what behavior those lines represent
3. Assess whether behavior is critical, nice-to-have, or dead code
4. If critical, document as gap with priority
5. If nice-to-have, document as medium/low priority
6. If dead code, note for potential removal (separate phase)

## Example Gap Entry

```markdown
### Gap: Admin-Only Endpoint Authorization Failure

**Location**: `crates/api/src/handlers/users.rs:145-152`

**Uncovered Behavior**:
When a bidder operator attempts to call `DELETE /api/users/:id`,
the authorization check should fail with 403 Forbidden.

**Why It Matters**:
Authorization failures are security boundaries. Untested authorization
paths represent unvalidated security assumptions.

**Priority**: Critical

**Estimated Complexity**: Simple (add test with bidder session, verify 403)

**Test Strategy**:

- Create bidder operator session
- Attempt DELETE on valid user
- Assert 403 response
- Assert user still exists
```

## Completion Conditions

- Coverage report generated and committed (HTML output in docs or ignored directory)
- `COVERAGE_GAPS.md` created with prioritized gaps
- Critical gaps (authorization, validation) fully enumerated
- Each gap has clear description and priority
- Gaps are ranked by both priority and complexity
- Document passes markdown linting
- Git commit contains coverage report reference and gap analysis only

## Dependencies

- Phase 27F must be complete (tests must be reliable before measuring coverage)

Unreliable tests produce unreliable coverage data.

## Blocks

- Phase 27H (Coverage Gap Remediation)

Cannot write tests until gaps are identified and prioritized.

## Execution Notes

### Coverage Thresholds

Do NOT set arbitrary coverage percentage goals. Focus on:

- 100% coverage of authorization checks
- 100% coverage of validation error paths
- High coverage (>80%) of state transitions
- Reasonable coverage (>60%) of overall critical paths

Leave low-priority code at lower coverage if tests would be complex.

### Coverage Report Interpretation

llvm-cov shows:

- Green: Lines executed by tests
- Red: Lines never executed
- Yellow: Partially covered (e.g., match arms)

Focus on red lines in critical modules, not total percentage.

### Integration vs Unit Coverage

- Unit tests provide line-level coverage
- Integration tests may cover code but obscure gaps
- Prefer unit tests for authorization and validation paths
- Integration tests are acceptable for end-to-end workflows

Both contribute to coverage, but unit tests make gaps clearer.

### False Positives

Some uncovered lines may not need tests:

- Unreachable error paths (type system prevents)
- Debug/logging code
- Trivial conversions
- Panic paths that represent bugs, not expected behavior

Document these as "not applicable" rather than gaps.

### Machine-Readable Output

Generate lcov format for potential CI integration:

```bash
cargo llvm-cov --lcov --output-path coverage.lcov
```

This enables:

- Coverage tracking over time
- Automated gap detection
- CI failure on coverage regression (future work)

### Documentation Quality

The gap analysis must be detailed enough that Phase 27H can implement tests without re-analyzing modules.

Include:

- Exact file and line numbers
- Description of untested scenario
- Expected behavior
- Why it matters (security, correctness, audit)
- Suggested test approach

This enables efficient test implementation in 27H.
