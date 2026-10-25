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
| **Drag Scrubber** | `ui.scrubber("Exposure", &mut val, min, max)` | Blender-style draggable numeric slider |
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

### Dynamic EditorRegistry
Applications register custom editors without modifying framework internals:
```rust
let mut registry = EditorRegistry::new();
registry.register_factory("custom_viewport", "Custom Viewport", "Rendering", || {
    Box::new(MyCustomEditor::new())
});
let workspace = WorkspaceHost::with_registry(registry);
```

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
Run the verified interface demonstrations built with public Kathamo APIs:
```bash
# 1. 2D Creative Workstation (Layers, Inks, Brush Dynamics, Export Modal)
cargo run --example creative_2d -p kathamo

# 2. 3D Spatial DCC Workstation (3D Viewport Blit, Outliner, PBR Material Inspector)
cargo run --example spatial_3d -p kathamo

# 3. Procedural Flow Graph (Typed Pins, Spline Wires, Groups, Node Inspector)
cargo run --example procedural_editor -p kathamo

# 4. NLE Video Workstation (Video Monitor, Multi-Track Scrubbable Timeline, Clips)
cargo run --example video_editor -p kathamo

# 5. General Desktop Application (Telemetry HUD, Form Controls, Console Log, Migration Dialog)
cargo run --example desktop_app -p kathamo
```

### Agent Inspection & Verification (`InterfaceInspector`)
Verify UI layouts, widget bounds, and element states deterministically:
```rust
let mut snap = InterfaceInspector::new_snapshot("main_window", viewport);
InterfaceInspector::add_editor(&mut snap.root, "canvas", "2D Canvas", canvas_rect);
InterfaceInspector::add_dock_tab(&mut snap.root, "tab_layers", "Layers", tab_rect, true);

assert!(snap.find("canvas").is_some());
assert_eq!(snap.root.find_by_role("DockTab").len(), 1);
```

