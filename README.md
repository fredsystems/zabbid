# ZAB Bid

## Overview

This project is the authoritative backend system for managing and validating domain-critical data.
It exposes an API used by a frontend UI, but **all decisions, validation, and rule enforcement live exclusively in the backend**.

The frontend is a presentation layer only. It does not interpret, infer, or enforce business rules.

## Core Principles

This system is built around the following non-negotiable principles:

### Backend Authority

- The backend is the single source of truth.
- The frontend never makes domain decisions.
- All validation happens server-side.

### Data Integrity First

- Correctness and auditability always outweigh performance or convenience.
- Every state-changing action must be validated against domain rules.
- Invalid actions must fail loudly and explicitly.

### Explicit State Transitions

- State changes are never implicit.
- Transitions must be intentional, validated, and observable.
- Any action that modifies state must be traceable.

### Auditability by Design

- Every state change must produce an auditable record.
- Audit history is immutable once written.
- Historical data is never silently overwritten or deleted.

## Domain Model Philosophy

The domain rules (e.g. leave bidding, time-off constraints, eligibility rules) are:

- **Centralized** — implemented in one authoritative place
- **Explicit** — no hidden logic or implicit assumptions
- **Tested** — both success and failure paths are covered
- **Explainable** — failures return structured, meaningful errors

If a rule cannot be clearly validated or explained, it does not belong in the system yet.

## API Design Goals

- APIs expose _capabilities_, not internal structure
- Requests are validated atomically
- Errors are structured, not stringly-typed
- Breaking changes require explicit migration paths

Public APIs are treated as contracts.

## Testing Philosophy

Tests in this project are not just for correctness — they encode intent.

- Every code path must be tested, including failures
- Tests should describe _why_ behavior exists, not just _what_ it does
- Domain rules must be tested independently of transport (HTTP, UI, etc.)

If behavior is important enough to implement, it is important enough to test.

## What This Project Is Not

- A frontend or UI framework
- A place for speculative features
- A system that infers intent
- A performance-first optimization playground

## Working on This Project

Before making changes:

1. Understand the domain rule being affected
2. Identify the authoritative place for that logic
3. Ensure the change preserves auditability
4. Add or update tests to reflect intent

If any of the above is unclear, stop and ask.

For contributor rules, AI-specific constraints, and architectural guardrails, see **AGENTS.md**.

## Definition of Done (Agent-Facing)

A change is considered complete only if all of the following are true:

- The domain rule being affected is explicitly identified
- All state transitions are validated and observable
- The change preserves or improves auditability
- All new behavior (including failures) is covered by tests
- No domain logic is duplicated across layers
- No assumptions are made about future rules or behavior

If any of these cannot be satisfied, the change is incomplete.

In such cases, stop and ask for clarification before proceeding.

## Testing & Infrastructure Philosophy

Tests in this project encode domain intent and system contracts.

- Tests define correct behavior, not just implementation details
- Failing tests indicate a violation of an explicit rule or invariant
- Infrastructure-related failures (e.g. missing services, missing tools) are not test failures

If a test requires additional infrastructure (such as a database or external service),
that requirement must be made explicit and provisioned intentionally.
