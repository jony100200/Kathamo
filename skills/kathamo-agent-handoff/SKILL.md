---
name: kathamo-agent-handoff
description: Coordinate auditable, bounded Kathamo UI implementation and review across independent coding agents, preserving public API compatibility and private local material.
---

# Kathamo Agent Handoff

## Operating modes

- `AUDIT_ONLY`: inspect the latest code and report verified gaps; no edits.
- `SPECIFICATION_ONLY`: propose contracts, architecture and scoped files; no edits.
- `SCOPED_IMPLEMENTATION`: implement approved changes and verify locally.
- `REVIEW_ONLY`: independently audit and reproduce defects.

Never silently broaden a scoped task or claim unexecuted tests passed. Framework changes must benefit more than one consuming application.

## Mandatory task header

```text
Objective:
Mode:
Owner acceptance criteria:
Repository / branch / HEAD SHA:
Affected public APIs and crates:
Existing behavior to preserve:
Local-only information boundary:
Planned files / dependency changes:
Verification strategy:
```

## Workflow

1. Inspect the repository's actual state, exported APIs and existing guidance.
2. Identify the smallest coherent change and the consuming application contract.
3. Agree on stable IDs, state ownership, input/focus semantics, errors and backward compatibility.
4. Implement independently testable increments; never rebuild a working subsystem without evidence.
5. Run local build/lint and relevant verification when available; record failures honestly.
6. Check the exact staged file list for private information and accidental generated artifacts.
7. Write a concise handoff so another agent can resume without private chat history.

## Required handoff

```text
Change and rationale:
Files changed (public):
Public API changes and compatibility:
Build/lint/native interaction outcomes:
Known defects and follow-up:
Local-only evidence path (not to be uploaded):
Rollback:
Staged-file/privacy check:
Ready for owner review: YES / NO
```

## Hard stops

- Personal R&D, references, screenshots, generated test files or agent logs being staged or transmitted.
- A nonfunctional control shown as working, or a broken main editing workflow.
- Unauthorized code publish, branch merge or dependency/network action.
- Persistent state lost during editor/layout switching.
- Runtime/model execution accidentally added to generic UI framework core.

Use companion **framework**, **ux-architect**, **ui-builder**, and **visual-qa** skills when their area is affected. Kathamo remains application-independent and uses its own designs and terminology.
