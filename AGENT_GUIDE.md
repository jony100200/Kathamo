# Kathamo (কাঠামো) — Agent & Developer Architecture Guide

> **Official UI Framework Guide for AI Coding Agents (Dolil, Antigravity, Codex) and Human Developers.**  
> Maintained by **Kalponic Studio**. Distributed under the **MIT License**.

---

## 1. What is Kathamo?

**Kathamo (কাঠামো)** is a lightweight, commercial-grade, native Rust UI framework developed by Kalponic Studio. It is engineered specifically for desktop workstation applications (such as **Ojon**, **Cholpo**, and **Chobi**) that must run side-by-side with heavy local AI workloads (LLMs, VLMs, Diffusion models) on 1080p and 1440p displays.

### The Three Core Rules
1. **Zero-Allocation Per Frame**: Never allocate heap vectors or strings inside render loops. All text commands and draw instructions use preallocated arenas and cached texture atlases.
2. **Never Block the UI Thread**: Expensive background work (model scanning, quantization, tensor slicing, downloading) runs on worker threads via `TaskController`. The UI thread remains pinned at 120+ FPS with sub-3ms frame times.
3. **Declarative Layout Ergonomics**: Agents must describe layouts via high-level nested closures (`ui.horizontal`, `ui.columns`, `ui.glass_card`, `ui.model_card`), never manually calculating raw pixel bounding boxes.

---

## 2. Design System & Aesthetics (Linear & Raycast Standards)

Kathamo follows modern visual luxury guidelines inspired by Linear, Raycast, and Apple HIG:

### Surface Depth Ladder (No Pure Blacks)
| Layer | Hex | Palette Property | Purpose |
| :--- | :--- | :--- | :--- |
| **Canvas** | `#0B0E14` | `palette.canvas` | Inset background, deep viewport well |
| **Surface** | `#121721` | `palette.surface` | Panels, sidebar backgrounds, cards |
| **Elevated** | `#1B2231` | `palette.elevated` | Hovered cards, active tabs, buttons |
| **Elevated High**| `#242D41` | `palette.elevated_high` | Modals, floating dropdowns, popovers |

### Dual-Stroke Specular Elevation
Cards and active controls feature a **1px hairline top specular highlight** (`rgba(255, 255, 255, 0.08)`) combined with a **1px border** (`palette.border`). This simulates physical studio lighting without expensive multi-pass blurred drop shadows.

---

## 3. The `KathamoUi` Builder API

Applications interact exclusively with `KathamoUi<'a>` (type alias for `KolponaUi<'a>`):

```rust
use ui_core::{Direction, SizeConstraint, Vec2};
use ui_themes::BilingualLabel;
use ui_widgets::*;

fn render_my_screen(ui: &mut KathamoUi) {
    // 1. Section Header with Bengali Script Support
    ui.bilingual_heading(&BilingualLabel::new("Model Studio", "মডেল স্টুডিও"));
    
    // 2. Horizontal Row of Actions
    ui.horizontal(36.0, |row| {
        if row.button("Run Benchmark").clicked() {
            // trigger action
        }
        row.button_secondary("Cancel");
        row.status_pill("CUDA 12.4 Active", true);
    });

    ui.separator();
}
```

---

## 4. Declarative Layouts

### A. Horizontal Rows (`ui.horizontal` / `ui.row`)
Places widgets side-by-side. Inside the closure, the cursor automatically advances horizontally (`x += width + spacing`).
```rust
ui.horizontal(32.0, |row| {
    row.button("Action 1");
    row.button("Action 2");
    row.status_pill("Online", true);
});
```

### B. Multi-Column Grids (`ui.columns`)
Divides available horizontal space using `SizeConstraint` (Fixed, Flex, or Clamped):
```rust
ui.columns(
    500.0, // container height
    &[SizeConstraint::Fixed(280.0), SizeConstraint::Flex(1.0)], // sidebar 280px, rest flex
    |col_idx, col| {
        if col_idx == 0 {
            col.heading("Sidebar");
            col.search_bar(&mut my_search_query, "Filter items...");
        } else {
            col.heading("Main Content");
            col.label("Inspection and configuration panel");
        }
    },
);
```

### C. Glassmorphic Cards (`ui.glass_card`)
Wraps inner controls in a modern translucent card with dual-stroke specular lighting:
```rust
ui.glass_card(160.0, |card| {
    card.heading("Quantization Parameters");
    card.toggle("FlashAttention-2 Kernel", &mut is_fa2_enabled);
    card.scrubber("Context Window", &mut ctx_len, 512.0, 8192.0);
});
```

---

## 5. Ready-to-Use Component Catalog

| Widget | Signature | Description |
| :--- | :--- | :--- |
| **Primary Button** | `ui.button("Start Job")` | Prominent accent CTA button |
| **Secondary Button** | `ui.button_secondary("Cancel")` | Subtle outline button |
| **Toolbar Button** | `ui.button_toolbar(StudioIcon::Play)` | Ghost icon button |
| **Toggle Switch** | `ui.toggle("CUDA Offload", &mut bool)` | iOS/macOS styled pill switch with sliding knob |
| **Checkbox** | `ui.checkbox("Snap to Grid", &mut bool)` | Standard studio checkbox |
| **Text Input** | `ui.text_input(&mut text, "Placeholder...")` | Text input box with focus ring & cursor |
| **Search Bar** | `ui.search_bar(&mut text, "Search...")` | Text input with embedded vector search icon |
| **Segmented Tabs** | `ui.tabs(&["GGUF", "AWQ", "EXL2"], &mut idx)` | Linear-style workspace tab switcher |
| **Dropdown** | `ui.dropdown("Profile", &mut idx, &options)` | Expandable option selector |
| **Status Pill** | `ui.status_pill("Ready", is_online)` | Pill badge with glowing status dot |
| **Model Card** | `ui.model_card(name, params, quant, is_sel)` | Selectable list item with badge tags |
| **Telemetry Stat** | `ui.stat_card("VRAM Pool", "11.8 GB", "74% Free")` | Telemetry metric display card |
| **Progress Bar** | `ui.progress(0.75)` | Sleek 6px progress indicator |
| **Drag Scrubber** | `ui.scrubber("Exposure", &mut val, min, max)` | Interactive draggable numeric value scrubber |
| **Vector Icon** | `ui.icon(StudioIcon::Sparkles)` | Cached zero-allocation Tabler SVG icon |

---

## 6. Real-World Application Blueprints

### Blueprint A: AI Quantization Workstation (The Ojon Pattern)
```rust
pub fn render_quantization_screen(ui: &mut KathamoUi, state: &mut WorkstationState) {
    // 1. Top Telemetry Bar
    ui.horizontal(72.0, |top| {
        top.stat_card("PRIMARY VRAM", "11.8 / 16.0 GB", "74% Active");
        top.stat_card("SECONDARY POOL", "24.0 GB", "Intel Arc B60");
        top.stat_card("SYSTEM MEMORY", "21.4 / 32.0 GB", "DMA Buffering");
    });
    
    ui.separator();

    // 2. Split Screen: Model List (320px) + Configuration (Flex)
    ui.columns(520.0, &[SizeConstraint::Fixed(320.0), SizeConstraint::Flex(1.0)], |idx, col| {
        if idx == 0 {
            col.heading("MODEL CHECKPOINTS");
            col.search_bar(&mut state.query, "Search models...");
            for (i, m) in state.models.iter().enumerate() {
                if col.model_card(&m.name, &m.params, &m.format, state.selected_idx == i).clicked() {
                    state.selected_idx = i;
                }
            }
        } else {
            col.heading("QUANTIZATION RECIPE");
            col.glass_card(180.0, |card| {
                card.tabs(&["GGUF", "AWQ", "EXL2"], &mut state.target_format);
                card.dropdown("Preset", &mut state.recipe_idx, &state.recipes);
                card.toggle("FlashAttention-2", &mut state.flash_attn);
                card.scrubber("Context Window", &mut state.ctx_len, 512.0, 8192.0);
            });
            
            col.glass_card(120.0, |job| {
                job.horizontal(24.0, |hdr| {
                    hdr.status_pill("Quantizing Shard 3/4", true);
                    hdr.label("Throughput: 142.8 MB/s");
                });
                job.progress(0.68);
                job.button("Start Quantization");
            });
        }
    });
}
```

### Blueprint B: Background Concurrency (Zero UI Freezes)
```rust
use ui_core::{TaskController, TaskId};

// In your application startup:
let (task_ctrl, task_reporter) = TaskController::new(TaskId(1), "Model Quantization");

// Background worker thread:
std::thread::spawn(move || {
    task_reporter.report_progress(0.25, "Quantizing layer 1..12...");
    // do expensive work
    task_reporter.report_progress(0.75, "Quantizing layer 13..24...");
    // finish
    task_reporter.complete("GGUF Q5_K_M conversion finished successfully");
});

// In your KathamoUi loop:
let receipt = task_ctrl.receipt();
ui.stat_card(&receipt.name, &format!("{:.0}%", receipt.status.progress() * 100.0), "Active Worker");
```

---

## 7. Verification & Benchmarking Workflow

When an agent writes or modifies Kathamo code:

1. **Syntax & Type Check**:
   ```bash
   cargo check --workspace
   ```
2. **Unit & Regression Suite**:
   ```bash
   cargo test --workspace
   ```
3. **Automated Headless Stress Benchmark (1000 frames @ 1440x900)**:
   ```bash
   cargo run --release -j 2 -p ui-lab -- --benchmark
   ```
   - Performance Gate: `p95 <= 16.7 ms` (target: ~2.5 ms, 0 stalls).
   - Generates visual QA screenshots in the artifacts directory (`ui_lab_frame.ppm`, `ojon_workstation_frame.ppm`).

---

### Visual check (do this for every UI change)

1. `KATHAMO_SNAPSHOT=<file>.png` + run the example: renders one GPU frame headless and exits.
2. Look at the PNG; fix overlap, clipping and alignment; re-shoot.
3. Long labels: `ui_render::fit_text(text, px, strong, max_width)` ends them with an ellipsis instead of cutting mid-word.
4. Hover, toggle and slide states: `ui_core::motion::animate_bool(motion::rect_key("kind", rect), on, motion::HOVER)` returns 0..1 for `Color::lerp`. The native runner redraws only while a spring moves; tests and snapshots snap instantly.
5. Time-based UI (toasts, timers): call `motion::wake_at(deadline)` instead of redrawing continuously; the runner sleeps until then.
6. Tab rows and multi-state selectors: `ui_widgets::tab_strip` (measured widths, sliding pill, shared `tab_rects`/`hit` so clicks match drawing). Never size tabs with `len() * px`.
7. Code, logs and aligned numbers: `DrawCommand::Code` (monospace face) or `ui_widgets::code_view::render_code_line` for token colouring from the theme.
8. Screen regions: describe them once with `kathamo::flex::Flex` (CSS flexbox via taffy) and use the solved rects for both drawing and hit-testing. Never recompute `left_w`/`top_offset` in each input handler.
9. Floating surfaces (menus, popovers, toasts): `DrawCommand::Backdrop { rect, radius, tint }` gives frosted glass; draw the shadow first and the border after.

## 8. High-Level Kathamo SDK (`kathamo` Crate)

The `kathamo` crate provides a unified, production-grade SDK for composing complex desktop applications without directly assembling low-level draw calls or manual coordinate geometry.

### Single-Import Master Prelude
Applications and agents import everything via one canonical line:
```rust
use kathamo::prelude::*;
```

### Application Lifecycle & Trait (`KathamoApp`)
Implement `KathamoApp` to receive window lifecycle events, resize notifications, theme tokens, and input dispatch:
```rust
pub struct MyStudioApp {
    pub workspace: WorkspaceHost,
    pub menu_bar: MenuBar,
    pub notifications: NotificationHost,
    pub dialogs: DialogHost,
}

impl KathamoApp for MyStudioApp {
    fn title(&self) -> &str {
        "My Native Workstation"
    }

    fn default_size(&self) -> (u32, u32) {
        (1600, 1000)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let palette = ctx.palette().clone();
        let bounds = ctx.bounds();

        // 1. Menu Bar
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        self.menu_bar.render(&mut ctx.cmd_buf, menu_rect, &ctx.pointer, &palette);

        // 2. Docked Workspace Host
        let ws_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.render(&mut ctx.cmd_buf, ws_rect, &ctx.pointer, &palette);

        // 3. Floating Overlays
        self.dialogs.render(&mut ctx.cmd_buf, bounds, &ctx.pointer, &palette);
        self.notifications.render(&mut ctx.cmd_buf, bounds, &palette);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, ctx: &mut AppContext) -> bool {
        let bounds = ctx.bounds();
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        if menu_rect.contains(pos) {
            if let Some(action_id) = self.menu_bar.handle_mouse_down(pos, button, menu_rect) {
                // handle action
                return true;
            }
        }
        let ws_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.handle_mouse_down(pos, button, ws_rect, &ctx.pointer)
    }
}
```

### WorkspaceHost & Preset Workspaces
`WorkspaceHost` provides full docking, splitting, tabbing, maximize/restore, and multi-workspace layout switching:
```rust
let mut workspace = WorkspaceHost::new();

// Switch between standard presets:
workspace.switch_workspace(0); // Creative 2D (Canvas + Layers + Inspector)
workspace.switch_workspace(1); // Spatial 3D (3D Viewport + Outliner + PBR Inspector + Asset Browser)
workspace.switch_workspace(2); // Procedural Graph (Node Editor + Parameters + Library)
workspace.switch_workspace(3); // Video Editing (Monitor Preview + Multi-Track Timeline + Clips)
workspace.switch_workspace(4); // General Desktop (Telemetry HUD + Console + Service Browser)
```

### Custom Workspaces and Reusable Editors
Register an editor **type**, then compose unique pane **instances**. One editor type can appear in several independent panels:
```rust
let mut registry = EditorRegistry::new();
registry.register_factory("custom_viewport", "Viewport", "Rendering", || {
    Box::new(MyCustomEditor::new())
});
let mut tree = DockTree::new(
    "left", DockTabPanel::new("view_a", "Perspective").with_factory("custom_viewport")
);
tree.split_leaf(
    "left", "root_split", "right", SplitAxis::Horizontal, 0.5,
    DockTabPanel::new("view_b", "Top").with_factory("custom_viewport"),
);
let workspace = WorkspaceHost::from_workspace(
    registry, Workspace::new("editor", "Scene Editor", tree),
);
```
Use `WorkspaceHost::with_preset(registry, id, name, WorkspacePreset::Spatial3D)` for one ready-made layout. Use `WorkspaceHost::with_registry` only when you want all five standard workspaces.

An application implements `KathamoApp`, forwards pointer input to its `WorkspaceHost`, then starts its real OS window with `run_native(app)`. The application supplies viewport image buffers, editor data, and backend functionality; Kathamo supplies UI chrome and input infrastructure.

### Fluent Declarative Layouts (`LayoutBuilder`)
Compose forms, cards, scrubbers, and controls with zero coordinate math:
```rust
let mut layout = LayoutBuilder::new(panel_rect, &mut ctx.cmd_buf, &ctx.pointer, &palette);
layout.card(|ui| {
    ui.heading("Render Settings");
    ui.separator();
    ui.toggle("Hardware Ray Tracing", &mut enable_rt);
    ui.scrubber("Sample Count", &mut samples, 1.0, 1024.0, 1.0);
    ui.space(4.0);
    if ui.button("Bake Lighting") {
        // trigger bake
    }
});
```

### 5 Standard Framework Demonstrations
Run an interactive native window by passing `--native` (without it, each example runs headless checks):
```bash
# 1. 2D Creative Workstation (Layers, Inks, Brush Dynamics, Export Modal)
cargo run --example creative_2d -p kathamo -- --native

# 2. 3D Spatial DCC Workstation (3D Viewport Blit, Outliner, PBR Material Inspector)
cargo run --example spatial_3d -p kathamo -- --native

# 3. Procedural Flow Graph (Typed Pins, Spline Wires, Groups, Node Inspector)
cargo run --example procedural_editor -p kathamo -- --native

# 4. NLE Video Workstation (Video Monitor, Multi-Track Scrubbable Timeline, Clips)
cargo run --example video_editor -p kathamo -- --native

# 5. General Desktop Application (Telemetry HUD, Form Controls, Console Log, Migration Dialog)
cargo run --example desktop_app -p kathamo -- --native
```

### Visual Designer document (shared with the native Designer)

```rust
let mut ui = DesignDocument::template(DesignTemplate::GameStudio);
let id = ui.add(DesignKind::Button, "Run",
    Rect::from_min_size(Vec2::new(300.0, 100.0), Vec2::new(120.0, 30.0)));
ui.find_mut(&id).unwrap().label = "Build".into();
ui.save("interface.kui").unwrap();
let reloaded = DesignDocument::load("interface.kui").unwrap();
let rust_source = reloaded.export_rust();
```

Run `cargo run -p kathamo --example visual_designer` to edit the same `.kui` file visually. Agents should edit the document by stable component IDs rather than generating pixel-position-only screenshots. The Designer now exports UI layout, responsive placement, and state/action binding IDs. Use `DesignRuntime` to instantiate real widgets and inspect or activate stable IDs. Agents may call `set_text_by_id` and `activate`; the consuming application remains responsible for interpreting and authorizing emitted `UiEvent` actions.

```rust
let mut runtime = DesignRuntime::new();
runtime.sync(&reloaded);
runtime.set_text_by_id(&reloaded, &id, "Hello").ok(); // only TextInput accepts text
let actions = runtime.take_events();
```

For complex node UIs, validate and save `GraphDocument::from_editor(&editor)`; use `render_guided` for a readable step view or `apply_to` for the full existing `NodeEditor`. The two presentations share IDs, but Kathamo never executes the graph.

Local verification after source edits: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo test --workspace`, then `cargo run -p kathamo --example visual_designer -- --headless` and `cargo run -p kathamo --example interactive_ui`. Native pointer/keyboard/IME review is still required.

### Agent Inspection & Verification (`InterfaceInspector`)
Verify UI layouts, widget bounds, and element states deterministically:
```rust
let mut snap = InterfaceInspector::new_snapshot("main_window", viewport);
InterfaceInspector::add_editor(&mut snap.root, "canvas", "2D Canvas", canvas_rect);
InterfaceInspector::add_dock_tab(&mut snap.root, "tab_layers", "Layers", tab_rect, true);

assert!(snap.find("canvas").is_some());
assert_eq!(snap.root.find_by_role("DockTab").len(), 1);
```


## 9. Developer Reference

The README is the introduction; this section holds the hands-on detail.

### Architecture & Crates

```
Kathamo/
├── core/       ← Layout math, anchors, docking tree, modifiers, task signals
├── platform/   ← winit window lifecycle, DPI scaling, multi-touch/mouse, AccessKit
├── render/     ← wgpu hardware swapchain, infallible softbuffer CPU safe mode
├── themes/     ← Fluent 2 scale, Studio Graphite tokens, bilingual Bengali labels
├── text/       ← HarfBuzz/rustybuzz shaping, FontRegistry, lazy fallback fonts
├── widgets/    ← Commercial studio controls, Tabler vector atlas, KolponaUi builder
├── testkit/    ← Frame timer, p95 benchmark recorder, offscreen pixel testing
├── sdk/        ← Public KathamoApp, native windows, editor workspaces and UI API
└── lab/        ← Interactive UI test harness and benchmark runner (ui-lab)
```

---

### SDK Quick Start

Kathamo provides the **user interface**; each application supplies its own painting, 3D, video, audio or other processing logic.

Launch an interactive native SDK example:

```bash
cargo run -p kathamo --example creative_2d -- --native
cargo run -p kathamo --example spatial_3d -- --native
cargo run -p kathamo --example procedural_editor -- --native
cargo run -p kathamo --example video_editor -- --native
cargo run -p kathamo --example desktop_app -- --native
```

Omit `--native` to run the existing headless checks. Import `kathamo::prelude::*` and call `run_native(app)` to start your own application.

- **Image/video/3D viewport:** Use `CanvasHost::new_texture_host(id, (width, height))` and `ctx.set_image_rgba8(id, width, height, rgba)`. The same pixels render on GPU or CPU. For zero-copy wgpu textures, implement `KathamoApp::prepare_gpu`, create textures on `gpu.device()`, and call `gpu.register_external_view(id, view)`.
- **Dock layout:** Use `WorkspaceHost::with_preset(...)` or `from_workspace(...)`. Persist with `workspace.save_layouts_to(path)` / `load_layouts_from(path)`.
- **Detachable panels:** In the 3D example, drag the viewport tab outside the dock or press **Ctrl+Shift+D**. Consuming apps can use `WorkspaceHost::detach_to_window`, `KathamoApp::update_window`, and `window_closed`; close the child window to reattach it.
- **Keyboard/focus:** `WorkspaceHost::handle_key_down` supports Tab/Shift+Tab pane traversal; forward text and IME to the workspace from `KathamoApp`.
- **Layout:** `LayoutBuilder` supports custom padding, spacing, orientation and weighted `flex_columns`.
- **Agent API:** `InterfaceInspector::capture_workspace(&workspace, bounds)`, `InterfaceInspector::hit_test(&snapshot, pos)`, `workspace.activate_panel(id)`.

Kathamo supplies the UI only; applications implement actual painting, scene rendering, video decoding, and other behavior. Full cross-platform interaction and accessibility still need testing.


#### Visual Designer (included)

```bash
cargo run -p kathamo --example visual_designer
# Optional: open a specific saved project
cargo run -p kathamo --example visual_designer -- --file path/to/interface.kui
# Non-graphical smoke check for agents
cargo run -p kathamo --example visual_designer -- --headless
```

Build application UI layouts visually using the left-side component palette, canvas, hierarchy and editable property inspector. Choose an **Empty / Game Studio / 3D DCC / Digital Painting / Chat** starter layout, drag to place and reposition components, resize using the bottom-right handle, and edit properties by clicking their values. Scroll to zoom. **Preview** uses live Kathamo controls, not the decorative design shapes; buttons, toggles, text fields, graph widgets and other implemented controls receive real input. Set **Layout**, **State key**, and **Action ID** in the property inspector to configure responsive placement and application bindings.

**Save/Open** read and write a versioned `.kui` document. **Export Rust** writes an editable `build_ui()` function using the public `DesignDocument` model. Default files go into `~/KathamoProjects` (or `%USERPROFILE%\\KathamoProjects` on Windows), not the repository. Click the project-path field to change the destination.

Keyboard: **Ctrl+S** save, **Ctrl+O** open, **Ctrl+E** export, **Ctrl+Z/Y** undo/redo, **Delete** remove, **arrow keys** nudge; **Enter/Escape** apply/cancel a property. Developers and agents can use `DesignDocument::add`, `add_child`, `find_mut`, `set_parent`, `serialize`, `deserialize`, and `render` directly.

The Designer exports layouts, state/action IDs and responsive placement metadata. `DesignRuntime` reads those documents and emits `UiEvent` actions; the consuming application handles actual behaviour. Register custom components with `ComponentRegistry`, inspect and invoke elements through stable IDs, and populate built-in `NodeEditor`, `ChatTranscript`, `MarkdownView`, `CanvasHost`, and other controls through the runtime.

```bash
# Public-SDK interactive chatbot-shaped UI (UI only; no model backend)
cargo run -p kathamo --example interactive_ui
cargo run -p kathamo --example interactive_ui -- --native

# Verify all crate examples and tests after pulling
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
```

`GraphDocument` saves and validates node positions, typed ports and links, groups, cycles and duplicate sockets. Its guided card renderer and the existing advanced `NodeEditor` use identical node IDs; application execution remains external to the framework. The framework also includes layout grid/wrap solvers, IME-aware input, event binding and a streaming-chat presentation widget.

**Verification:** these latest integrations require local Rust compilation and native interaction review. Rendering a widget does not prove application-specific functionality, and the lightweight Markdown widget is not a complete rich-text editor.

---

### Real-World Workstation Demonstration: Ojon (ওজন)

Kathamo includes a full-fidelity implementation of **Ojon (ওজন)** — Kalponic Studio's local AI model quantization workstation ([github.com/jony100200/Ojon](https://github.com/jony100200/Ojon)).

Run the standalone example:
```bash
cargo run -p ui-lab --example ojon_workstation
```

Or open the interactive UI Lab and switch to the **Ojon Studio** tab:
```bash
cargo run -p ui-lab
```

---

### Benchmark Gates & Measured Evidence (1,000 Frames @ 1440×900)

| Metric | Measured Result | Target Gate | Verdict |
| :--- | :--- | :--- | :--- |
| **p50 Frame Time** | **1.22 ms** | < 8.3 ms (120 Hz) | ✅ Excellent |
| **p90 Frame Time** | **2.64 ms** | < 16.7 ms (60 Hz) | ✅ Excellent |
| **p95 Frame Time** | **2.79 ms** | < 16.7 ms (60 Hz) | ✅ **PASS (6.0× Headroom)** |
| **p99 Frame Time** | **3.26 ms** | < 16.7 ms (60 Hz) | ✅ Excellent |
| **Frame Stalls (>16.7ms)** | **0** | 0 stalls | ✅ Perfect |
| **RAM (private, `desktop_app --native`)** | **~170–180 MB** | — | Measured; mostly GPU driver |
| **Discrete-GPU VRAM** | **0 MB** (UI runs on the integrated GPU) | Leave VRAM for models | ✅ |
| **Idle CPU (no input, after hover animations)** | **0.00 %** over 10 s | 0 % | ✅ |
| **GPU Recovery Fallback** | **Instant Safe Mode** | Zero panic / crash | ✅ Seamless Recovery |
| **Presentation Flash** | **ZERO frames** | Zero white screen | ✅ Hidden until first paint |


RAM, VRAM and idle CPU were measured on Windows 11, on a laptop with an integrated and a discrete GPU, using Windows performance counters.

#### GPU selection

Kathamo asks for the **low-power (integrated) GPU** first, so its UI does not occupy the discrete GPU or its VRAM. Apps that render heavy 3D in their viewport can opt into the discrete GPU:

```bash
WGPU_POWER_PREF=high cargo run -p kathamo --example kathamo_forge
```

#### Glass viewport demo

A real wgpu 3D scene (depth buffer, 4x MSAA, lit meshes, emissive orbs, grid floor) embedded through the zero-copy external-texture path, with frosted-glass panels that blur the live 3D view beneath them. Drag to orbit, scroll to zoom.

```bash
cargo run --release -p kathamo --example glass_viewport
```

Measured on an integrated GPU at 1920x974: 16.7 ms per frame (vsync-locked 60 fps), 0 MB on the discrete GPU, about 26-43 % of one CPU core while animating.

#### Headless screenshots

Any app started with `run_native` writes one GPU-rendered frame to a PNG and exits when `KATHAMO_SNAPSHOT` is set. No desktop session is needed, and motion is settled first so images are deterministic:

```bash
KATHAMO_SNAPSHOT=forge.png cargo run --release -p kathamo --example kathamo_forge
```

---

### Platform Support

Kathamo is developed primarily on Windows. **Linux has not yet been tested by the maintainer**, but it is expected to work because the framework uses cross-platform Rust windowing and rendering libraries.

**Linux users:** If you'd like to use Kathamo on Linux, please try it on your machine:

```bash
cargo check --workspace
cargo run -p ui-lab
```

If you encounter any problems, please open a GitHub issue with your Linux distribution, graphics hardware, and the error details. Linux compatibility is **community-testing welcome**, not yet maintainer-verified.


#### Kolpona native shell showcase

A studio-style desktop shell using Kathamo's native GPU surfaces, soft shadows, and warm charcoal design tokens. Select one of six application cards and toggle its inspector. This is a **UI demonstration only**; agent runtimes and application services are not simulated.

```bash
cargo run -p kathamo --example kolpona_shell -- --native
cargo run -p kathamo --example kolpona_shell
```

## 10. UI Craft Playbook

How to make a Kathamo screen look and feel professional, and prove it. Each `### Craft:` section stands alone, so an agent can load only the one it needs.

### Craft: loop

Never call a UI done from code alone. Work in this loop:

1. **Measure first.** Run the app; record RAM, discrete-GPU VRAM and idle CPU (10 s, no input). Numbers beat impressions.
2. **Look.** `KATHAMO_SNAPSHOT=shot.png` + run the app writes one GPU frame and exits. Open the PNG.
3. **Critique like a user.** List what is wrong: overlap, clipping, misalignment, binary state flips, placeholders, missing glyph boxes, duplicated headers.
4. **Fix the framework, not the screen,** when the cause is shared (a widget, a component, the renderer).
5. **Re-shoot and compare.** Every fix gets a new snapshot.
6. **Verify live** for motion: sample real screen pixels over time; a spring shows intermediate colours, a flip does not.
7. **Measure again.** A visual win that costs idle CPU or VRAM is not a win.
8. **Report only measured claims.** Remove any sentence you did not measure.

### Craft: layout

- Describe screen regions once with `kathamo::flex::Flex` and use the solved rects for **both drawing and hit-testing**. Never recompute `left_w`, `top_offset` or panel sizes inside each input handler; copies drift apart and clicks land on the wrong thing.
- Tabs, menus and segmented controls are sized from **measured text** (`ui_render::measure_text`), never `label.len() * px`. Use `ui_widgets::tab_strip` for tab rows and multi-state selectors.
- One header per panel. If a widget draws its own toolbar (for example `HierarchyTree`), do not draw another above it; set its `title` instead.
- Keep floating surfaces (toasts, popovers) clear of toolbars and status bars.

### Craft: text

- Labels that can overflow use `ui_render::fit_text(text, px, strong, max_width)`; they end in an ellipsis, never cut mid-word or spill past a border.
- Code, logs and aligned numbers use `DrawCommand::Code` (monospace) or `ui_widgets::code_view::render_code_line` (token colours from the theme).
- Use `StudioIcon` vector icons, not emoji or symbol characters (➤ 📋 ⏸) that the bundled fonts may not contain; missing glyphs render as boxes.
- Centre text vertically from the rect (`rect.center().y - px * 0.6`), not with fixed offsets.

### Craft: motion

- Every hover, press, toggle, selection and slide is a spring: `motion::animate_bool(motion::rect_key("kind", rect), on, motion::HOVER)` returns 0..1; blend with `Color::lerp`. Positions use `motion::animate(key, target, motion::SLIDE)`.
- Enter transitions use `motion::animate_from(key, 0.0, 1.0, ..)`.
- Never count frames for timers: idle apps draw no frames. Use `Instant` plus `motion::wake_at(deadline)`.
- `wants_continuous_redraw` is `true` only while something genuinely plays (a 3D view rotating, a simulation running). The runner redraws while springs move and then sleeps at 0 % CPU.

### Craft: depth

- Surfaces climb the ladder `canvas → surface → elevated → elevated_high`; no pure black.
- Shadows are `DrawCommand::SoftShadow`, never a flat black rectangle behind a card.
- Floating surfaces (menus, popovers, toasts, viewport HUDs) are frosted glass: shadow, then `DrawCommand::Backdrop { rect, radius, tint }`, then a 1 px rim. Glass needs content behind it to read as glass.
- The blur has no opacity: fade a glass panel by drawing a fading solid card during enter/exit, glass only when fully shown.

### Craft: states

Every interactive control shows: rest, hover (spring), pressed, focused (animated accent ring with soft glow, as `render_text_input` does), disabled (muted, no hover). One focus ring per field; a container around a focused input stays neutral.

### Craft: honesty

- No fake data on screen: no "[Thumb]" placeholders, invented metrics or simulated agents. Without a preview, show a file-type icon. Unknown values say so.
- Stats shown in a UI are measured at runtime (frame time, triangle counts, real memory of real buffers).
- Machine details (GPU names, paths) appear only at runtime, never in code, docs or public screenshots.

### Craft: performance

- The UI prefers the integrated GPU; heavy 3D opts into the discrete GPU with `WGPU_POWER_PREF=high`.
- Shaped text is cached per frame; do not re-shape in tight loops yourself.
- Glass costs about 1.3 frames of VRAM only while a backdrop is on screen.
- After any visual change, re-measure idle CPU after a hover sweep: it must return to 0 %.

### Craft: viewport

A real 3D or video viewport renders into a wgpu texture in `KathamoApp::prepare_gpu` and is shown with `gpu.register_external_view(id, view)` plus `DrawCommand::Image` (zero copy). Output sRGB-encoded colour into `Rgba8Unorm`. See `sdk/examples/glass_viewport` for depth, MSAA, lighting and glass HUD panels over live 3D.

### Craft: ux

Design for the person's goal, not the feature list.

- Write the screen's goal as one sentence before drawing anything. Every element must serve it; remove what does not.
- One primary action per screen, in the accent colour. Everything else is secondary or ghost.
- Show the common path; hide the advanced one one click away (progressive disclosure: accordion, "More", inspector).
- Good defaults beat options. A setting nobody changes is noise.
- Prefer undo over "Are you sure?". Confirm only what cannot be undone (delete files, publish, spend money).
- Keep people oriented: the selected item, the current mode and the current file are always visible.
- Speed is UX: input responds in the same frame, transitions settle in about 150-250 ms, never block the UI thread (use `TaskController`).

### Craft: hierarchy

- Spacing comes from the 4 px grid only: `SPACE_1` 4, `SPACE_2` 8, `SPACE_3` 12, `SPACE_4` 16, `SPACE_6` 24, `SPACE_8` 32 (`ui_themes::metrics`). Related things sit closer than unrelated things.
- Type comes from `FluentTypographyScale`: Caption 12, Body 14, Section 16, PanelTitle 20, WorkspaceTitle 28, HeroTitle 36. At most three text sizes per panel; one title per panel.
- Weight and colour carry hierarchy before size does: strong for titles, `text` for content, `text_muted` for labels and metadata.
- Align to shared edges. A column of controls shares one left edge and one label width.
- Reading text stays under `WORKSPACE_READABLE_WIDTH` (760). Pro tools may be dense (rows 28-44 px) but never cramped: every cluster keeps its padding.

### Craft: color

- Use palette tokens only (`canvas`, `surface`, `elevated`, `elevated_high`, `border`, `text`, `text_muted`, `accent`, `success`, `warning`, `danger`). A hard-coded colour breaks the other theme.
- Accent means "act here" or "selected". If everything is accent, nothing is.
- Status colours always come with a word or icon (colour-blind users, glanceability).
- Contrast is measured, not eyeballed: `fg.contrast_ratio(bg)` at least 4.5 for body text, 3.0 for large text and control outlines. Check both dark and light themes.

### Craft: feedback

- Every action answers: hover and press respond instantly (springs), clicks change something visible, long work shows progress with a cancel.
- Background results arrive as a toast (`NotificationHost`): a short title, one line of detail, the right level (info, success, warning, error).
- Errors say what happened and what to do next, in plain words. Never show a raw error code alone.
- Empty states teach: say what belongs here and offer the action that fills it ("No assets yet — Import files").
- Loading has a shape: show the layout with muted placeholder blocks, not a blank panel or a spinner in the void.

### Craft: input

- Every command is reachable from the keyboard: a shortcut or the command palette. Show shortcuts in menus.
- Tab moves focus in reading order, Esc closes or cancels, Enter confirms, arrows move within lists and tabs.
- Focus is always visible (the animated focus ring).
- Hit targets are at least 24 px; toolbar buttons use `SIZE_TOOL_BUTTON` (32).
- Draggable things look draggable on hover (cursor, handle, highlight) and show where they will drop.
- Right-click opens a context menu of the same commands as the main menu, never different ones.

### Craft: accessibility

- Give controls accessibility nodes (`KolponaUi::with_accessibility`): role, name and focus, so screen readers and agents can see the interface.
- Respect reduced motion: when the user asks for it, call `motion::set_instant(true)` (springs snap) and use `motion_seconds(true)`.
- Respect the system scale factor; never draw text below Caption (12) at 100 %.
- Bilingual by design: `BilingualLabel` and the Bengali font are first-class; never cut a Bengali cluster with byte slicing.

### Craft: review

Answer every question about a fresh snapshot (`KATHAMO_SNAPSHOT`) before calling a screen done. A "no" is a task.

1. Can you say the screen's goal and point to its one primary action in two seconds?
2. Is anything overlapping, clipped, cut mid-word, or spilling past a border?
3. Are there boxes instead of characters, placeholder text, or invented numbers?
4. Do edges line up, and does spacing follow the 4 px grid?
5. At most three text sizes per panel, with titles, content and labels clearly different?
6. Is the selected item, the current mode and the focus visible?
7. Do empty, loading and error states exist and say what to do?
8. Do floating surfaces avoid covering toolbars and status bars?
9. Does the text pass contrast in both themes?
10. After a hover sweep, does idle CPU return to 0 %, and is the discrete GPU still free?

### Craft: screen

Recipe for a new screen:

1. Goal sentence and primary action.
2. Regions with `kathamo::flex::Flex` (header, body, side panels, status) — one layout for drawing and input.
3. Fill regions from the component catalog before writing custom drawing.
4. Add every state: hover, press, focus, selected, disabled, empty, loading, error.
5. Snapshot, run the review checklist, fix, re-shoot.
6. Run the audit (`kathamo.audit` in Pothik) and clear its findings.
7. Measure RAM, VRAM and idle CPU; report only what you measured.
