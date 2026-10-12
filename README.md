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

## 3. Benchmark Gates & Measured Results

- **Cold Launch:** 730 ms
- **Warm Launch:** < 10 ms
- **p95 Frame Time:** 2.58 ms (6.4× headroom at 60 Hz)
- **100,000-Item Virtual List:** Steady 120+ FPS with kinetic fling inertia
- **Presentation Flash:** ZERO frames (window stays hidden until first buffer commits)

---

## 4. Licenses & Open Source Acknowledgements

- **Kathamo Core**: Dual-licensed under [MIT](LICENSE-MIT) or Apache-2.0.
- **Third-Party Acknowledgements**: Complete attribution and licenses for all bundled fonts (`Inter`, `Space Grotesk`, `Noto Sans Bengali`, `JetBrains Mono`, `IBM Plex Mono`), icons (`Tabler Icons`), and foundation crates (`wgpu`, `winit`, `rustybuzz`, `accesskit`, `resvg`) are detailed in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

