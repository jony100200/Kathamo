---
name: kathamo-visual-qa
description: Perform local, evidence-based UX, interaction, accessibility, layout and performance verification on applications built with Kathamo.
---

# Kathamo Visual and Interaction Verification

## Evidence policy

Do not equate source presence, screenshot quality or successful compilation with functional correctness. State separately what was read, built, launched and manually or automatically exercised. Never invent measurements.

All private references, native screenshots, golden images, generated fixtures, standalone test files, performance traces, logs and R&D reports must remain **outside Git**, under `$KS_KATHAMO_PRIVATE` or another owner-approved local directory. Do not publish or upload them.

## Verification levels

1. **Source:** follow public API calls to actual handlers and state changes.
2. **Structure:** widget IDs, ancestry, bounds, clipping, visibility, enabled status and focus.
3. **Interaction:** real mouse, keyboard, input method, drag/drop, workspace and persistence actions.
4. **Appearance:** native captures examined locally at supported sizes and DPI.
5. **Performance:** on-machine frame timings, redraw rate, allocation/CPU/GPU/memory and recovery behavior.

## Minimum local interaction matrix

| Area | What to verify |
| --- | --- |
| Buttons | single activation, hold, release outside, disabled handling |
| Pointer | captured drag, hover, pan, scroll, nested region routing |
| Keyboard | Tab, Shift+Tab, Enter, Space, Escape, navigation, shortcuts |
| Text | typing, selection, paste, composition/IME, English and Bengali |
| Docking | split, resize, tab, move, close, maximize/restore, saved layout |
| Canvas | clipping, zoom anchor, resize, focus, app input handoff |
| Graph | type validation, wire drag, grouping, multi-selection, error state |
| Inspector | bound values, mixed selections, validation, undo/redo |
| Layout | compact window, 1080p, 1440p, high DPI and wide displays |
| Reliability | close/reopen, empty data, missing assets, error, long sessions |
| Runtime | real GPU/CPU paths where supported and no UI thread stalls |
| Privacy | explicit Git allowlist, no local/private artifacts staged |

## Automated structural checks

Flag duplicated stable IDs, missing accessibility labels, clipped controls, hit targets behind overlays, invalid rectangle sizes, unreachable focus controls, bindings that don't update actual data, unhandled commands, and model/task status disconnected from source state.

## Report format (local only)

```text
HEAD SHA / executable build:
OS, display, DPI, renderer:
Interface and scenario:
Source path / API contract:
Native action performed:
Expected result:
Observed result:
PASS / FAIL / BLOCKED / NOT RUN:
Evidence location (PRIVATE):
Regression risk and needed fix:
```

Reject a polished but nonfunctional UI. Mark tests NOT RUN when native execution is unavailable. Visual QA can use approved local screenshots; no illustrative image generation is required.
