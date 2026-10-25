# Kathamo SDK Readiness Audit — 2026-10-24

**Scope:** Source-level inspection of GitHub `main` at `78aee6b60a7be352b313e9616390ace47b7d0b0d`. The branch containing this report also proposes two narrowly scoped SDK correctness fixes. This is **not** a completed GUI usability test or a freshly executed benchmark.

## Product boundary

Kathamo provides native Rust application UI/UX infrastructure: reusable layout, drawing, input, focus, accessibility, editor hosts, docking, theming, and inspection. A consuming application owns painting, CAD/3D rendering, audio/video processing, projects and asset formats, model execution, and business actions. A UI demonstration is not proof of any application engine.

## Verified source inventory

- Workspace crates: `core`, `platform`, `render`, `themes`, `text`, `widgets`, `testkit`, `lab`, `sdk`.
- Public `kathamo` SDK exports `KathamoApp`, `AppContext`, `HeadlessAppRunner`, `WorkspaceHost`, `Editor`, editor registry, layout builder, and UI components.
- Implemented building blocks include dock-tree geometry and interactive chrome; splitters/tabs; canvas; node graph; hierarchy; property inspector; timeline; menus/dialogs/notifications; assets and console; theme and text infrastructure; AccessKit-tree conversion and UI inspection helpers.
- Five `sdk/examples/*.rs` demonstrations exercise creative 2D, spatial 3D, procedural graphs, video editing, and desktop UI. Their `main` functions use `HeadlessAppRunner`, count commands, and assert mock dialog/inspector state; they **do not launch native windows**.
- `lab/src/main.rs` and `lab/src/window.rs` contain a separate native `winit` launcher for UI Lab.
- `CanvasHost` emits `DrawCommand::Image { texture_id, uv_rect, tint }`. The current GPU renderer matches `texture_id: _` and tessellates through its existing UI atlas pipeline; the CPU renderer ignores `texture_id` and draws a tinted fill. **External image texture integration is not complete.**
- No `.github/workflows` files appear in the inspected tree. The README publishes benchmark results, but this audit did not rerun or validate them.
- The README explicitly states that maintainer testing on Linux has not yet occurred.

## Blockers, with observable exit criteria

### P0 — Complete, public native SDK runtime

**Finding:** `KathamoApp` provides an update function and mouse-down hook; the SDK exposes only a headless runner. Native lifecycle and input/event pumping live separately in UI Lab.

**Required:** Supply `NativeAppRunner` (or an equivalent stable public API) using `winit` and the selected rendering backend. Route logical DPI resize, focus, close, keyboard down/up, text/IME, pointer down/move/up, wheel, drag capture and redraw invalidation. Make app-owned commands render to an actual window.

**Done when:** A new consumer importing only `kathamo` can launch each example as an interactive native window on Windows and Linux; controls, docking, keyboard focus and resizing work without importing `ui-lab` internals. Include automated synthetic-event tests and human smoke-test records.

### P0 — Real viewport/image compositing

**Finding:** `texture_id` is currently unused by `GpuBackend` image rendering and by the CPU image fallback. The image command's existence does not demonstrate a usable 2D/3D/video frame.

**Required:** Explicit texture upload/import/registration API, ownership and lifetime contract, UV mapping, scissor/clip support, sampler selection, alpha/color-space handling, resize, device-loss/recreation, missing-texture fallback and backend parity. Prefer a typed texture handle over an undocumented numeric ID.

**Done when:** Render a known checkerboard RGBA image with correct pixels/UV, render an externally produced viewport frame in the SDK-native app, recover from lost textures safely, and compare GPU/CPU screenshots. No fake placeholder should be claimed as a real image.

### P0 — Integrated interaction dispatch

**Finding:** Lower-level pointer and text utilities exist, but `KathamoApp` and `Editor` do not provide a complete, documented key/text/IME/focus/shortcut dispatch contract. A drawn control is not necessarily interactive.

**Required:** Stable event sequence, hit testing and focus ownership, keyboard tab order, hover/press/release/capture, IME composition, shortcut priority and modal handling. Avoid triggers solely from render-time polling.

**Done when:** All representative controls work with mouse and keyboard (including Bengali IME/text), focus cannot leak through a modal, and a test can replay input without a live desktop.

## Priority 1 — Production behavior

1. **Workspace serialization and recovery.** Versioned save/restore for dock tree, active workspace, tab order, maximized panel, editor viewport states, unknown/renamed panels and invalid/corrupt state; explicit application-owned storage.
2. **Independent editor instances.** Separate editor *type* registration from unique pane instance identity. The current `WorkspaceHost` editor maps use panel IDs and factories keyed by IDs, complicating two simultaneously open canvases or viewports of the same type. Show two live viewport instances with independent state.
3. **Robust docking.** Test panel drag source/destination, same-leaf reorder, no-op/cancel, tab overflow, minimum panel sizes, cross-window/multi-monitor movement and non-destructive failures. Do not remove a source panel before destination insertion is guaranteed.
4. **Accessibility bridge.** Drive the available AccessKit semantics from the actual native window and widget tree; keyboard focus, roles, names, values, descriptions, screen reader and high-contrast tests.
5. **Platform/quality gates.** Add CI for `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo test --workspace`, Clippy, and Windows/Linux compatibility. Test CPU fallback, DPI and GPU recovery. Confirm performance on representative interactive layouts rather than relying solely on a benchmark table.

## Priority 2 — Professional UI quality and ecosystem

- Constraint-based composition and responsive minimum/maximum dimensions; stable resizing and overflow, scrolling, virtualization and typography-aware measurement in panels, menus, tables and inspectors.
- Consistent design tokens for typography, spacing, color, borders, elevation, hover/press/focus/disabled states; native-feeling interaction density; light/dark/high-contrast themes and scalable icons.
- Polished menus, context menus, shortcuts, searchable command palette, toolbars, popovers, dialogs, file dialogs, property grids, editable trees, multi-select lists and drag/drop.
- Public component cookbook, screenshot-backed interactive examples, migration notes, a versioned API contract, and an example standalone consuming repository.
- Visual QA matrix covering small laptop through high-DPI workstation/multi-monitor displays, long/localized labels, empty/busy/error states and keyboard-only workflows.
- Optional visual UI designer/inspector belongs later. First make the code-first API and event/runtime paths dependable.

## Recommended execution order

1. Native runner + event/focus contract; launch one non-headless SDK example and validate it.
2. Texture registry/import and real viewport compositing, including CPU fallback and device loss.
3. Complete key/IME/modals/accessibility paths and integration tests.
4. Workspace/editor-instance persistence and resilient docking.
5. Interaction, screenshot, cross-platform, and performance CI.
6. Theme/polish/documentation/authoring ergonomics and broader component gallery.

## Changes proposed with this audit

- `sdk/src/app.rs`: `HeadlessAppRunner` honors `KathamoApp::default_size()` and has a regression assertion.
- `sdk/src/workspace.rs`: each active editor draw is bounded by a balanced `ClipPush`/`ClipPop`; regression assertions ensure clip pairs are balanced.
- The remaining P0/P1 work is **not** implemented by this audit.

## Verification commands (not run by this audit)

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo run -p ui-lab
cargo run -p kathamo --example creative_2d
cargo run -p kathamo --example spatial_3d
```

**Important:** the `kathamo` examples above currently run headless. They will not open a native UI until the native SDK runner is implemented.

Status of this report: **source inspection complete; build, native manual QA, OS compatibility and benchmarks not executed in the audit environment**.
