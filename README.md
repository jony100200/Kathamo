<p align="center">
  <img src="assets/KathamoFramework.png" alt="Kathamo (কাঠামো) — Native Rust Studio UI Framework" width="100%" />
</p>

# Kathamo (কাঠামো) — Native Rust Studio UI Framework

**Kathamo (কাঠামো)** is the native, high-performance UI & GUI framework powering the Kalponic Studio ecosystem (**Kolpona, Cholpo, Chobi, Drisho, Rupayon, Jogot, Dolil, Pothik**).

> In traditional Bengali art and idol sculpture, **কাঠামো (Kathamo)** is the sacred bamboo and wood armature built first. Only upon a strong, balanced Kathamo can the clay, colors, ornaments, and final divine form emerge. 

Kathamo bridges the gap between modern Web/React aesthetics (Linear, Vercel, Radix, Tailwind UI) and native Rust execution: sub-3 ms frames, zero CPU while idle, the integrated GPU by default (the discrete GPU's VRAM stays free), and no white flash on launch.

---

## 1. Architecture & Crates

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

## 2. Why Web UIs Look Great & How Kathamo Replicates Them

| Modern Web (React / Vue / Tailwind) | Traditional Desktop (egui / Qt) | **Kathamo (কাঠামো) in Rust** |
| :--- | :--- | :--- |
| **Frosted-Glass Popovers (`backdrop-filter`)** | Opaque menus | **`DrawCommand::Backdrop`: dual-Kawase blur at reduced resolution, only allocated while used** |
| **Flexbox Layout** | Hand-placed pixel offsets | **`kathamo::flex` (CSS flexbox via taffy) — one solved layout for drawing and input** |
| **Layered Dual-Stroke Elevation** | Flat solid 1px borders | **Subtle rim stroke (`rgba(255,255,255,0.06)`) + ambient occlusion** |
| **Spring Micro-Interactions** | 0ms binary hover/click flips | **Critically damped springs (`ui_core::motion`), frame-rate independent, zero cost once settled** |
| **Typography-First Architecture** | Unshaped pixel/bitmap fonts | **HarfBuzz bilingual shaping with cached layouts, ellipsis fitting, and a monospace face with token-coloured code** |
| **Declarative Component Ergonomics** | Imperative immediate loops | **Composable `KolponaUi` builder API with state responses** |
| **300 MB – 1.5 GB Electron Bloat** | Low memory but clunky visuals | **~170 MB RAM for a full GPU app, no browser engine** |

---

## 3. Declarative Component Ergonomics (`KathamoUi`)

Building professional, Linear-grade interfaces in Kathamo is declarative, type-safe, and effortless for both human developers and AI coding agents:

```rust
use ui_core::{SizeConstraint, Vec2};
use ui_themes::BilingualLabel;
use ui_widgets::*;

// Declarative layout: Master/Detail split with glassmorphic cards
ui.columns(
    520.0, // height
    &[SizeConstraint::Fixed(320.0), SizeConstraint::Flex(1.0)],
    |col_idx, col| {
        if col_idx == 0 {
            col.heading("MODEL REPOSITORY");
            col.search_bar(&mut query, "Filter checkpoints...");
            if col.model_card("Qwen-2.5-Coder-7B", "7.6B Params", "GGUF Ready", true).clicked() {
                // handle model selection
            }
        } else {
            col.heading("QUANTIZATION RECIPE");
            col.glass_card(180.0, |card| {
                card.tabs(&["GGUF", "AWQ", "EXL2"], &mut selected_tab);
                card.toggle("FlashAttention-2 Kernel", &mut is_fa2_enabled);
                card.scrubber("Context Window", &mut ctx_len, 512.0, 8192.0);
            });
            col.progress(0.68);
            col.button("Start Quantization Job");
        }
    },
);
```

For complete patterns, blueprints, and best practices, see the [Agent & Developer Architecture Guide](AGENT_GUIDE.md).

---

## 4. Real-World Workstation Demonstration: Ojon (ওজন)

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

## 5. Benchmark Gates & Measured Evidence (1,000 Frames @ 1440×900)

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

---

RAM, VRAM and idle CPU were measured on Windows 11, on a laptop with an integrated and a discrete GPU, using Windows performance counters.

### GPU selection

Kathamo asks for the **low-power (integrated) GPU** first, so its UI does not occupy the discrete GPU or its VRAM. Apps that render heavy 3D in their viewport can opt into the discrete GPU:

```bash
WGPU_POWER_PREF=high cargo run -p kathamo --example kathamo_forge
```

### Headless screenshots

Any app started with `run_native` writes one GPU-rendered frame to a PNG and exits when `KATHAMO_SNAPSHOT` is set. No desktop session is needed, and motion is settled first so images are deterministic:

```bash
KATHAMO_SNAPSHOT=forge.png cargo run --release -p kathamo --example kathamo_forge
```

---

## 6. Platform Support

Kathamo is developed primarily on Windows. **Linux has not yet been tested by the maintainer**, but it is expected to work because the framework uses cross-platform Rust windowing and rendering libraries.

**Linux users:** If you'd like to use Kathamo on Linux, please try it on your machine:

```bash
cargo check --workspace
cargo run -p ui-lab
```

If you encounter any problems, please open a GitHub issue with your Linux distribution, graphics hardware, and the error details. Linux compatibility is **community-testing welcome**, not yet maintainer-verified.

---

### Kolpona native shell showcase

A studio-style desktop shell using Kathamo's native GPU surfaces, soft shadows, and warm charcoal design tokens. Select one of six application cards and toggle its inspector. This is a **UI demonstration only**; agent runtimes and application services are not simulated.

```bash
cargo run -p kathamo --example kolpona_shell -- --native
cargo run -p kathamo --example kolpona_shell
```

## 7. SDK Quick Start

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

---

### Visual Designer (included)

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

## 8. Licenses & Open Source Acknowledgements

- **Kathamo Core**: Licensed under [MIT](LICENSE).
- **Special Thanks & Third-Party Notices**: Full attribution, gratitude, and complete license texts for all bundled fonts (`Inter`, `Space Grotesk`, `Noto Sans Bengali`, `JetBrains Mono`, `IBM Plex Mono`), vector icons (`Tabler Icons`), and foundation crates (`wgpu`, `winit`, `rustybuzz`, `accesskit`, `resvg`, `softbuffer`) are documented in [ACKNOWLEDGEMENTS.md](ACKNOWLEDGEMENTS.md).


