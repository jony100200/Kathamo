//! Showcase tabs: Docking Overview, Godot Anchors/M3, Infinite Canvas, Node Editor, Timeline, 100k List, Async Tasks, Renderer Recovery.

use crate::app::UiLabApp;
use ui_core::*;
use ui_render::{BackendKind, CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::*;

pub fn render_docking_overview_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    let dock = app.workspace.resolve(content_rect);

    // Left Panel: Workspace Hierarchy
    cmd_buf.push(DrawCommand::Rect {
        rect: dock.left_rect,
        fill: p.surface,
        radius: CornerRadius::ZERO,
        stroke: Stroke::new(1.0, p.border),
    });
    cmd_buf.push(DrawCommand::Text {
        text: "STUDIO HIERARCHY".into(),
        pos: Vec2::new(dock.left_rect.min.x + 14.0, dock.left_rect.min.y + 12.0),
        size: 11.5,
        color: p.accent,
        strong: true,
    });
    let items = [
        "Root Scene",
        "├── Camera2D",
        "├── Background_Atlas",
        "├── Player_Character",
        "└── Audio_Master",
    ];
    let mut hy = dock.left_rect.min.y + 36.0;
    for item in items {
        cmd_buf.push(DrawCommand::Text {
            text: item.into(),
            pos: Vec2::new(dock.left_rect.min.x + 16.0, hy),
            size: 12.0,
            color: p.text,
            strong: false,
        });
        hy += 26.0;
    }

    // Center: Protected Canvas
    app.canvas.render(cmd_buf, dock.canvas_rect, p);

    // Right: Property Inspector
    app.inspector.render(cmd_buf, dock.right_rect, p);

    // Bottom: Timeline Strip
    if dock.bottom_rect.height() > 0.0 {
        app.timeline.render(cmd_buf, dock.bottom_rect, p);
    }
}

pub fn render_anchors_and_material3_tab(
    _app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    cmd_buf.push(DrawCommand::Rect {
        rect: content_rect,
        fill: p.canvas,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });

    cmd_buf.push(DrawCommand::Text {
        text: "GODOT 4 ANCHOR/OFFSET SYSTEM & ANDROID MATERIAL 3 DESIGN TOKENS".into(),
        pos: Vec2::new(content_rect.min.x + 24.0, content_rect.min.y + 20.0),
        size: 14.0,
        color: p.accent,
        strong: true,
    });

    // Simulated Parent Container
    let parent_box = Rect::from_min_size(
        Vec2::new(content_rect.min.x + 24.0, content_rect.min.y + 55.0),
        Vec2::new(560.0, 380.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: parent_box,
        fill: p.surface,
        radius: CornerRadius::same(8.0),
        stroke: Stroke::new(1.0, p.border),
    });
    cmd_buf.push(DrawCommand::Text {
        text: "Parent Control Container (Normalized [0.0..1.0] Anchors)".into(),
        pos: Vec2::new(parent_box.min.x + 14.0, parent_box.min.y + 12.0),
        size: 11.5,
        color: p.text_muted,
        strong: false,
    });

    // Child 1: Top-Left Pinned
    let c1 = Anchors::TOP_LEFT.resolve(Offsets::new(16.0, 36.0, 150.0, 78.0), parent_box);
    let sb1 = StyleBox::new(p.elevated)
        .with_radius(6.0)
        .with_border(1.0, p.border);
    sb1.render(cmd_buf, c1, InteractionState::Default, p.accent);
    cmd_buf.push(DrawCommand::Text {
        text: "Anchors::TOP_LEFT".into(),
        pos: Vec2::new(c1.min.x + 12.0, c1.min.y + 12.0),
        size: 11.5,
        color: p.text,
        strong: true,
    });

    // Child 2: Centered Modal Dialog
    let c2 = Anchors::CENTER.resolve(Offsets::centered(Vec2::new(200.0, 65.0)), parent_box);
    let sb2 = StyleBox::new(p.accent).with_radius(8.0);
    sb2.render(cmd_buf, c2, InteractionState::Default, p.accent);
    cmd_buf.push(DrawCommand::Text {
        text: "Anchors::CENTER Modal".into(),
        pos: Vec2::new(c2.min.x + 20.0, c2.min.y + 24.0),
        size: 12.0,
        color: p.canvas,
        strong: true,
    });

    // Child 3: Bottom-Wide Pinned Toolbar
    let c3 = Anchors::BOTTOM_WIDE.resolve(Offsets::new(16.0, -52.0, -16.0, -14.0), parent_box);
    let sb3 = StyleBox::new(p.elevated)
        .with_radius(6.0)
        .with_border(1.0, p.border);
    sb3.render(cmd_buf, c3, InteractionState::Default, p.accent);
    cmd_buf.push(DrawCommand::Text {
        text: "Anchors::BOTTOM_WIDE Toolbar Dock".into(),
        pos: Vec2::new(c3.min.x + 16.0, c3.min.y + 12.0),
        size: 12.0,
        color: p.accent,
        strong: true,
    });

    // Material 3 Surface Elevation & State Layers (Right Side)
    let m3_box = Rect::from_min_size(
        Vec2::new(content_rect.min.x + 610.0, content_rect.min.y + 55.0),
        Vec2::new(560.0, 380.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: m3_box,
        fill: p.surface,
        radius: CornerRadius::same(8.0),
        stroke: Stroke::new(1.0, p.border),
    });
    cmd_buf.push(DrawCommand::Text {
        text: "Material 3 Surface Elevation & State Layer Mathematical Tints".into(),
        pos: Vec2::new(m3_box.min.x + 14.0, m3_box.min.y + 12.0),
        size: 11.5,
        color: p.text_muted,
        strong: false,
    });

    // Column 1: 4 Levels of Surface Depth
    let mut ey = m3_box.min.y + 42.0;
    for level in 0..=3 {
        let r = Rect::from_min_size(Vec2::new(m3_box.min.x + 16.0, ey), Vec2::new(240.0, 38.0));
        let col = p.surface_at_elevation(level);
        cmd_buf.push(DrawCommand::Rect {
            rect: r,
            fill: col,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, p.border),
        });
        cmd_buf.push(DrawCommand::Text {
            text: format!("Surface Elevation Level {}", level),
            pos: Vec2::new(r.min.x + 12.0, r.min.y + 11.0),
            size: 11.5,
            color: p.text,
            strong: false,
        });
        ey += 48.0;
    }

    // Column 2: State Layers
    let states = [
        ("Default State", InteractionState::Default),
        ("Hovered (+8% Tint)", InteractionState::Hovered),
        ("Pressed (+14% Tint)", InteractionState::Pressed),
        ("Focused (2px Accent)", InteractionState::Focused),
    ];
    let mut sy = m3_box.min.y + 42.0;
    for (name, st) in states {
        let r = Rect::from_min_size(Vec2::new(m3_box.min.x + 280.0, sy), Vec2::new(255.0, 38.0));
        let sb = StyleBox::new(p.elevated)
            .with_radius(6.0)
            .with_border(1.0, p.border);
        sb.render(cmd_buf, r, st, p.accent);
        cmd_buf.push(DrawCommand::Text {
            text: name.into(),
            pos: Vec2::new(r.min.x + 12.0, r.min.y + 11.0),
            size: 11.5,
            color: p.text,
            strong: st == InteractionState::Focused,
        });
        sy += 48.0;
    }
}

pub fn render_infinite_canvas_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    app.canvas.render(cmd_buf, content_rect, p);
}

pub fn render_node_editor_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    app.node_editor.render(cmd_buf, content_rect, p);
}

pub fn render_timeline_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    app.timeline.render(cmd_buf, content_rect, p);
}

pub fn render_virtualized_list_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    app.virtual_list.render(cmd_buf, content_rect, p, |idx| {
        format!(
            "Item #{:06} | Kalponic Studio Creative Asset Token | Size: {:.1} KB",
            idx,
            (idx as f32 * 0.17) % 500.0
        )
    });
}

pub fn render_async_workers_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    cmd_buf.push(DrawCommand::Rect {
        rect: content_rect,
        fill: p.canvas,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });

    cmd_buf.push(DrawCommand::Text {
        text: "ASYNC WORKER POOL & TASK LIFECYCLE (0ms UI Thread Freeze Guarantee)".into(),
        pos: Vec2::new(content_rect.min.x + 24.0, content_rect.min.y + 24.0),
        size: 14.0,
        color: p.accent,
        strong: true,
    });

    let spawn_btn_rect = Rect::from_min_size(
        Vec2::new(content_rect.max.x - 220.0, content_rect.min.y + 16.0),
        Vec2::new(190.0, 32.0),
    );
    let btn_hover = spawn_btn_rect.contains(app.pointer.position);
    cmd_buf.push(DrawCommand::Rect {
        rect: spawn_btn_rect,
        fill: if btn_hover {
            p.accent_hover
        } else {
            p.accent
        },
        radius: CornerRadius::same(6.0),
        stroke: Stroke::NONE,
    });
    cmd_buf.push(DrawCommand::Text {
        text: "+ Spawn Worker Task".into(),
        pos: Vec2::new(spawn_btn_rect.min.x + 16.0, spawn_btn_rect.min.y + 9.0),
        size: 12.0,
        color: p.canvas,
        strong: true,
    });

    let mut ty = content_rect.min.y + 60.0;
    for ctrl in &app.task_controllers {
        let receipt = ctrl.receipt();
        let card_rect = Rect::from_min_size(
            Vec2::new(content_rect.min.x + 24.0, ty),
            Vec2::new(content_rect.width() - 48.0, 75.0),
        );

        cmd_buf.push(DrawCommand::Rect {
            rect: card_rect,
            fill: p.surface,
            radius: CornerRadius::same(8.0),
            stroke: Stroke::new(1.0, p.border),
        });

        cmd_buf.push(DrawCommand::Text {
            text: format!("Task #{}: {}", receipt.id.0, receipt.name),
            pos: Vec2::new(card_rect.min.x + 16.0, card_rect.min.y + 14.0),
            size: 13.0,
            color: p.text,
            strong: true,
        });

        let status_str = match &receipt.status {
            TaskStatus::Queued => "Status: Queued".into(),
            TaskStatus::Running { progress, message } => {
                format!("Status: Running ({:.0}%) - {}", progress * 100.0, message)
            }
            TaskStatus::Completed { summary } => {
                format!("Status: Completed - {}", summary)
            }
            TaskStatus::Failed { error } => format!("Status: Failed - {}", error),
            TaskStatus::Canceled => "Status: Canceled".into(),
        };

        cmd_buf.push(DrawCommand::Text {
            text: status_str,
            pos: Vec2::new(card_rect.min.x + 16.0, card_rect.min.y + 42.0),
            size: 12.0,
            color: p.success,
            strong: false,
        });

        ty += 90.0;
    }
}

pub fn render_renderer_recovery_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    cmd_buf.push(DrawCommand::Rect {
        rect: content_rect,
        fill: p.canvas,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });

    let box_rect = Rect::from_min_size(
        Vec2::new(content_rect.min.x + 40.0, content_rect.min.y + 40.0),
        Vec2::new(600.0, 240.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: box_rect,
        fill: p.surface,
        radius: CornerRadius::same(10.0),
        stroke: Stroke::new(1.0, p.border),
    });

    cmd_buf.push(DrawCommand::Text {
        text: "GPU DEVICE LOSS RECOVERY & CPU SAFE-MODE FALLBACK".into(),
        pos: Vec2::new(box_rect.min.x + 20.0, box_rect.min.y + 20.0),
        size: 13.0,
        color: p.accent,
        strong: true,
    });

    cmd_buf.push(DrawCommand::Text {
        text: "The framework includes dual render pipelines:".into(),
        pos: Vec2::new(box_rect.min.x + 20.0, box_rect.min.y + 50.0),
        size: 12.0,
        color: p.text,
        strong: false,
    });
    cmd_buf.push(DrawCommand::Text {
        text: "• Hardware GPU: Direct DX12/Vulkan swapchain presentation (60-144+ FPS)".into(),
        pos: Vec2::new(box_rect.min.x + 20.0, box_rect.min.y + 75.0),
        size: 12.0,
        color: p.text_muted,
        strong: false,
    });
    cmd_buf.push(DrawCommand::Text {
        text: "• CPU Safe Mode: Pure memory rasterizer (infallible fallback during driver crash)"
            .into(),
        pos: Vec2::new(box_rect.min.x + 20.0, box_rect.min.y + 100.0),
        size: 12.0,
        color: p.text_muted,
        strong: false,
    });

    let status_msg = format!("Current Active Pipeline: {:?}", app.active_backend);
    cmd_buf.push(DrawCommand::Text {
        text: status_msg,
        pos: Vec2::new(box_rect.min.x + 20.0, box_rect.min.y + 135.0),
        size: 13.0,
        color: if app.active_backend == BackendKind::HardwareGpu {
            p.success
        } else {
            p.warning
        },
        strong: true,
    });

    let sim_btn_rect = Rect::from_min_size(
        Vec2::new(box_rect.min.x + 20.0, box_rect.min.y + 175.0),
        Vec2::new(290.0, 34.0),
    );
    let sim_hover = sim_btn_rect.contains(app.pointer.position);
    cmd_buf.push(DrawCommand::Rect {
        rect: sim_btn_rect,
        fill: if sim_hover {
            p.accent_hover
        } else {
            p.accent
        },
        radius: CornerRadius::same(6.0),
        stroke: Stroke::NONE,
    });
    cmd_buf.push(DrawCommand::Text {
        text: "Simulate Device Loss & Fallback".into(),
        pos: Vec2::new(sim_btn_rect.min.x + 16.0, sim_btn_rect.min.y + 10.0),
        size: 12.0,
        color: p.canvas,
        strong: true,
    });
}
