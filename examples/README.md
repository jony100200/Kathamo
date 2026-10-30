# Kathamo demos — build something worth showing

**12 runnable examples**, all visible at the repository root. These are complete Rust source examples, not screenshots or mock websites. Run from the repository root with Rust installed.

![Live 3D scene behind frosted-glass UI](../assets/glass_viewport.jpg)

## Start here

| Showcase | Demonstrates | Run |
| --- | --- | --- |
| [Glass Viewport](glass_viewport/) | Native wgpu 3D scene, orbit controls and live backdrop blur | `cargo run --release -p kathamo --example glass_viewport` |
| [Kathamo Chat](kathamo_chat/) | Native chat layout, composer, code blocks and artifact panel (mock messages, no model required) | `cargo run --release -p kathamo --example kathamo_chat` |
| [Kathamo Forge](kathamo_forge.rs) | Native editor shell, scene tree, docked inspector and tools | `cargo run --release -p kathamo --example kathamo_forge` |
| [Visual Designer](visual_designer.rs) | Native drag-and-drop visual interface designer and Rust export | `cargo run --release -p kathamo --example visual_designer` |
| [Kolpona Shell](kolpona_shell.rs) | Interactive Kolpona-style app launcher preview | `cargo run -p kathamo --example kolpona_shell -- --native` |

## Other workspaces and proof-of-concepts

| Example | What it covers |
| --- | --- |
| [2D Creative Editor](creative_2d.rs) | Painting-style workspace layout, panels and actions |
| [3D Spatial Editor](spatial_3d.rs) | 3D modelling-style docking and inspection layout |
| [Video Editor](video_editor.rs) | Sequencer-style workspace layout |
| [Procedural Editor](procedural_editor.rs) | Node-graph workspace, export actions |
| [Desktop Application](desktop_app.rs) | General desktop controls, dialogs and diagnostics |
| [Interactive UI](interactive_ui.rs) | Bound input, buttons and bilingual text |
| [Agent TUI](agent_tui.rs) | Terminal UI for agent activity and review |

Run any with `cargo run -p kathamo --example <name>`. The 2D, 3D, video, procedural and desktop workspace samples primarily exercise **headless UI state and layout**; they are not full creative applications or finished editors. `interactive_ui` defaults to a headless interaction test and supports `-- --native`.

### Automated visual checks

Use `KATHAMO_SNAPSHOT` to capture a supported native example as a PNG without a manual screenshot. For example, from PowerShell:

```powershell
$env:KATHAMO_SNAPSHOT = "kathamo-preview.png"
cargo run --release -p kathamo --example glass_viewport
```

### For developers

The Rust SDK remains in [`sdk/`](../sdk/). Its `Cargo.toml` explicitly lists these examples with paths into this gallery, preserving `-p kathamo --example ...`. Examples are demonstrations, and may use fixtures or simulated data. See the [developer guide](../AGENT_GUIDE.md) and [UX architect skill](../skills/kathamo-ux-architect/SKILL.md).

**No Electron. No browser runtime. Native Rust and GPU rendering.**
