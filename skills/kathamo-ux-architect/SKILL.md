---
name: kathamo-ux-architect
description: Design professional, coherent desktop interfaces with Kathamo, including creative canvases, editor workspaces, node graphs, inspectors, timelines, and general applications.
---

# Kathamo UX Architecture

## Goal

Make complex desktop work feel simple. The consuming application chooses which features exist; Kathamo provides consistent interaction behavior and composable visual structure. Do not hard-code one product's workflow into the framework.

## UX model

A user should understand three ideas without documentation:

1. **Work area** — the primary canvas, viewport, timeline, document or other editor.
2. **Context panels** — hierarchy, assets, properties, tools and status.
3. **Optional workflow graph** — explicit operations and connections when procedural control is useful.

A workspace is an arrangement of reusable editors and panels, not a separate application. Support focused editor, split editor and graph-focused arrangements, all sharing the same document identity.

## Interaction principles

1. Start with the user's primary job, not a widget catalog or dashboard.
2. Give the active creative area the most usable space.
3. Keep advanced features discoverable but collapsed until relevant.
4. Differentiate workspaces, documents, editors, views and panels.
5. Preserve document, selection, navigation, tool and layout state when switching or maximizing.
6. Panel resizing, docking, tabs, tooltips and shortcut hints must be predictable.
7. Use consistent focus, hover, pressed, selected, disabled, busy and error states.
8. Prefer context-aware property controls over arbitrary modal screens.
9. Keep graph editing optional when a direct toolbar or inspector action is more natural.
10. Make accessible keyboard paths and text editing first-class, including international input.
11. Actions must have clear progress, cancellation and error feedback; no decorative dead controls.
12. Do not require an AI agent or remote service for ordinary editing or navigation.

## Required local UX specification

Before building a view, record locally:

```text
Application and primary job:
Core document and source of truth:
Main editor / work area:
Left/right/bottom panels and initial visibility:
Toolbar, navigation and commands:
Workspace switch and maximize/restore behavior:
Pointer/keyboard/IME/drag interactions:
Graph visibility and relation to editor data:
Empty/loading/ready/error/disabled states:
Selection, undo and editing expectations:
Layout sizes, DPI and overflow behavior:
Save/reopen and state persistence:
Acceptance interactions and screenshots (PRIVATE):
```

## Visual language

Use Kathamo theme tokens rather than per-screen colors or copied interface treatments. Prioritize legible hierarchy, consistent spacing, balanced density, clearly bounded editor regions, restrained motion and meaningful color. A screenshot that appears polished but prevents the main job is a UX failure.

## Approval criteria

The user can open a document, edit, inspect, navigate, invoke tools, undo, adjust workspace, and return to the same work without confusion. Review with an actual native build whenever possible.

Local reference material and screenshots stay outside Git and are never uploaded. Original design decisions and terminology only.
