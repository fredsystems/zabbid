# Phase Planning & Execution Protocol

This document defines the **standard workflow** for planning, executing, pausing, resuming, and validating large engineering phases using agents.

Its goal is to make phases:

- mechanically executable by an agent
- safely resumable across context windows
- auditable and reviewable by a human
- resistant to scope drift and reinterpretation

Once adopted, the workflow should allow the human to say:

> “Here is the phase scope. Here is AGENTS.md. Do this.”

…and rely on the agent to plan, execute, pause, and resume correctly.

---

## Document Hierarchy & Authority

**Precedence order (highest → lowest):**

1. Phase Scope Document (e.g. `PHASE_28.md`)
2. Sub-Phase Documents (generated from the phase scope)
3. `AGENTS.md`
4. Phase Planning & Execution Protocol (this document)
5. Phase Working State Document

If any conflict exists, **higher-precedence documents always win**.

---

## Phase Lifecycle Overview

Each phase follows this lifecycle:

1. **Phase Scoping (Human + Agent)**
2. **Agent Planning Pass**
3. **Sub-Phase Execution**
4. **Pause / Resume (as needed)**
5. **Validation & Completion**

---

## 1. Phase Scope Document (Input)

Each phase begins with a **human-authored phase scope document**.

This document defines:

- Purpose
- Non-negotiable invariants
- Required outcomes
- Explicit non-goals
- Constraints and stop conditions

### Rules

- The phase scope document is authoritative
- The agent may not reinterpret or weaken invariants
- The agent may not introduce new goals
- The agent may not restructure the phase without approval

---

## 2. Agent Planning Pass

Given:

- the Phase Scope Document
- `AGENTS.md`
- this protocol

The agent must perform **one planning pass** before execution.

### Planning Pass Responsibilities

The agent must:

1. Analyze the codebase against the phase scope
2. Identify all required work
3. Divide work into **sub-phases** that:
   - are independently reviewable
   - fit within a single context window
4. Produce **sub-phase markdown documents** with:
   - Purpose
   - Scope
   - Explicit non-goals
   - Stop-and-ask conditions
   - Completion checklist
   - Risk notes

### Planning Output

- One updated Phase Overview (if needed)
- One markdown document per sub-phase

The agent must **stop after planning** and wait for approval before execution.

---

## 3. Sub-Phase Execution Rules

Each sub-phase is executed independently.

### Execution Rules

- Follow the sub-phase document strictly
- Do not bleed work across sub-phases
- Do not “pre-fix” future sub-phases
- If a decision is ambiguous, stop and ask

### Stop-and-Ask Conditions (Mandatory)

The agent must stop if execution requires:

- guessing domain intent
- inventing new rules
- weakening stated invariants
- changing behavior not covered by scope
- making irreversible architectural decisions

---

## 4. Phase Working State Document

Each phase maintains **one working state document** to support pause/resume.

File name:

```text
plans/PHASE_<N>/PHASE_<N>_WORKING_STATE.md
```

### Purpose

This document is a **factual execution ledger**, not a planning or reasoning artifact.

It exists to allow:

- context window exhaustion
- intentional pauses
- agent handoff
- safe resumption without reinterpretation

### What This Document Is

- A factual checkpoint
- A resumable execution state
- A handoff artifact

### What This Document Is Not

- A planning document
- A design document
- A reasoning log
- A memory surrogate

---

### Required Structure

```markdown
# Phase Working State

## Phase

- Phase: <number>
- Title: <phase title>

## Current Status

- Status: In Progress | Paused | Blocked | Complete
- Last Updated: YYYY-MM-DD
- Reason (if Paused/Blocked): <short factual reason>

## Active Sub-Phase

- Sub-Phase: <ID and title>
- State: Not Started | In Progress | Complete | Blocked

## Completed Sub-Phases

- [x] <Sub-phase ID and title>

## Work Completed

- Bullet list of concrete actions taken
- No commentary or rationale

## Outstanding Work

- Bullet list of remaining known tasks

## Known Failures / Breakages

- Failing tests, compile errors, or regressions
- Or explicitly: “None”

## Stop-and-Ask Items

- Items requiring human decision
- Or explicitly: “None”

## Resume Instructions

- Mechanical steps for the next agent:
  1. Continue sub-phase X
  2. Address outstanding work
  3. Run tests
  4. Update this document before pausing again
```
