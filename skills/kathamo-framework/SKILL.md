---
name: kathamo-framework
description: Design, extend and maintain Kathamo's reusable native Rust UI framework, including input, layout, rendering, editor hosting, graph widgets, accessibility and the public SDK.
---

# Kathamo Framework Engineering

## Mission

Kathamo is a **framework, not an application**. It must allow independent desktop applications to compose attractive, accessible, reliable UI and UX from stable public APIs. Application data processing, painting, simulation, media editing and model execution belong to the consuming application, not the framework.

## Before changing source

1. Verify repository root, branch, HEAD SHA, clean/dirty working tree and Cargo workspace membership.
2. Inspect current `Cargo.toml`, `AGENT_GUIDE.md`, public exports and relevant call sites. Do not trust historical API descriptions.
3. Trace the real window-event → input-dispatch → widget-state → layout → command-buffer → renderer → platform presentation path.
4. Identify application-visible compatibility, ownership, persistence and accessibility expectations.
5. Produce a small scoped change plan: affected files, API contract, behavior preserved, verification and rollback.

## Current module map (verify at HEAD)

- `core/`: IDs, layout, state, docking trees, actions, text-editing primitives.
- `platform/`: window events, pointer/keyboard input, DPI, accessibility.
- `render/`: draw commands, graphics backend and software fallback.
- `text/`: shaping and font registry.
- `themes/`: theme tokens, typography, visual metrics.
- `widgets/`: composable controls, graph, canvas, timeline, inspector and specialized widgets.
- `lab/`: executable framework demonstration surfaces.
- `testkit/`: reusable local verification primitives.

## Architectural rules

- **Public SDK first.** Consumers must be able to construct applications without editing internal framework crates. Favor coherent `KathamoApp`/`UiContext`/`EditorRegistry` concepts *only when actually implemented*. Never advertise proposed APIs as shipped.
- **Stable identity and ownership.** An editor document, its view state, its panel placement and its transient widget state are separate.
- **Single interaction authority.** One pointer activation per deliberate click, explicit pointer capture, keyboard focus and navigation, clipboard and text input including composition/IME.
- **Composable workspace host.** Dock/split/tab/maximize/restore editors; support app-owned viewports and textures with correct input routing, no document recreation.
- **Generic components.** Node graph UI, hierarchy tree, property inspector and timeline UI expose abstract data contracts; application-specific execution remains outside.
- **Semantic styling and accessibility.** Themes, interaction states, text measurement, screen-reader roles and DPI are framework responsibilities.
- **No fabricated behavior.** Buttons/actions, telemetry, progress and callbacks must reflect actual state. Never imply an incomplete input or rendering path works.
- **Preserve backend and consumer compatibility.** Extend existing implementations in verified increments. Measure rather than assume performance.

## Preferred implementation order

1. Native event wiring and widget input/focus correctness.
2. Reusable public application SDK and data/action contracts.
3. Canvas/viewport hosting and interactive docking.
4. Generic hierarchy, property binding, inspector and graph controls.
5. Integrated example use and API documentation.
6. Independent agent usability review.

## Verification

Run supported local formatting, build, lint and existing inline/unit tests where environment permits. Exercise real native interactions—click, hold, drag, type, paste, resize, dock, maximize, restore, save/reopen—and explicitly mark unrun checks. Keep QA screenshots, standalone tests, generated fixtures, logs and benchmarks **outside Git**, under `$KS_KATHAMO_PRIVATE`.

Before any commit, inspect an explicit changed-file allowlist. Never stage local-only materials. Do not push or publish without the owner's authorization.
