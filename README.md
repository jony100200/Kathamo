<p align="center">
  <img src="assets/KathamoFramework.png" alt="Kathamo (কাঠামো) — Native Rust Studio UI Framework" width="100%" />
</p>

# Kathamo (কাঠামো) — Native Rust Studio UI Framework

**Kathamo (কাঠামো)** is the native, high-performance UI & GUI framework powering the Kalponic Studio ecosystem (**Kolpona, Cholpo, Chobi, Drisho, Rupayon, Jogot, Dolil, Pothik**).

> In traditional Bengali art and idol sculpture, **কাঠামো (Kathamo)** is the sacred bamboo and wood armature built first. Only upon a strong, balanced Kathamo can the clay, colors, ornaments, and final divine form emerge. 

Kathamo bridges the gap between modern Web/React aesthetics (Linear, Vercel, Radix, Tailwind UI) and native Rust execution (120+ FPS, <15 MB RAM, sub-3ms frame times, 0ms white flash).

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
└── lab/        ← Interactive UI test harness and benchmark runner (ui-lab)
```

---

## 2. Why Web UIs Look Great & How Kathamo Replicates Them

| Modern Web (React / Vue / Tailwind) | Traditional Desktop (egui / Qt) | **Kathamo (কাঠামো) in Rust** |
| :--- | :--- | :--- |
| **Layered Dual-Stroke Elevation** | Flat solid 1px borders | **Subtle rim stroke (`rgba(255,255,255,0.06)`) + ambient occlusion** |
| **Spring Micro-Interactions** | 0ms binary hover/click flips | **Built-in spring physics & tweening interpolation** |
| **Typography-First Architecture** | Unshaped pixel/bitmap fonts | **HarfBuzz bilingual shaping + optical negative tracking** |
| **Declarative Component Ergonomics** | Imperative immediate loops | **Composable `KolponaUi` builder API with state responses** |
| **300 MB – 1.5 GB Electron Bloat** | Low memory but clunky visuals | **<15 MB RAM with buttery 120+ FPS native hardware performance** |

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
| **RAM Footprint** | **< 15 MB** | < 50 MB | ✅ Extremely Lightweight |
| **GPU Recovery Fallback** | **Instant Safe Mode** | Zero panic / crash | ✅ Seamless Recovery |
| **Presentation Flash** | **ZERO frames** | Zero white screen | ✅ Hidden until first paint |

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

## 7. Current SDK Maturity (October 24, 2026)

Kathamo is a reusable **application UI/UX framework**, not a 2D paint engine, 3D engine, video encoder, or AI runtime. Applications supply their own underlying functionality.

The public `kathamo` SDK now provides editor registry, workspace presets, dock layouts, reusable components and five application-style demonstrations. **Those five SDK examples currently run through `HeadlessAppRunner` and do not open native application windows.** UI Lab separately has a native demonstration window.

Key work still outstanding for Blender-, Unity-, Godot-, or Krita-class desktop interfaces:

- A public SDK native-window runner and complete keyboard/text/IME/focus dispatch.
- Real external GPU image/viewport texture binding and CPU image fallback (the current `DrawCommand::Image` renderer does not consume its `texture_id`).
- Layout/editor state persistence, robust multi-instance panes and docking, end-to-end accessibility, and cross-platform GUI verification.

Read the [source-based readiness audit and prioritized implementation gates](docs/SDK_READINESS_AUDIT_2026-10-24.md). The benchmark figures above are previously reported measurements, **not results freshly reproduced by the audit**.

---

## 8. Licenses & Open Source Acknowledgements

- **Kathamo Core**: Licensed under [MIT](LICENSE).
- **Special Thanks & Third-Party Notices**: Full attribution, gratitude, and complete license texts for all bundled fonts (`Inter`, `Space Grotesk`, `Noto Sans Bengali`, `JetBrains Mono`, `IBM Plex Mono`), vector icons (`Tabler Icons`), and foundation crates (`wgpu`, `winit`, `rustybuzz`, `accesskit`, `resvg`, `softbuffer`) are documented in [ACKNOWLEDGEMENTS.md](ACKNOWLEDGEMENTS.md).


